//! Extracts `hl7pet-core`'s public API surface directly from its Rust
//! source, via `syn` (spec 6000-python-bindings-automation, research.md #5)
//! — no nightly rustdoc-JSON toolchain, no compiled-artifact introspection.
//!
//! Only modules `crates/core/src/lib.rs` declares `pub mod` are walked —
//! that is the crate's actual reachable-from-outside surface. Within each
//! module, every top-level `pub fn`/`pub struct`/`pub enum`/`pub type` is
//! recorded unless it carries `#[doc(hidden)]` (FR-010's exclusion
//! convention: an item explicitly marked as not part of the crate's public
//! contract, even though `pub` for internal cross-module reasons).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use quote::quote;
use serde::{Deserialize, Serialize};
use syn::{Fields, Item, Visibility};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct StructDef {
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EnumDef {
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ModuleSurface {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub functions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub structs: BTreeMap<String, StructDef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub enums: BTreeMap<String, EnumDef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub types: BTreeMap<String, String>,
}

impl ModuleSurface {
    fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.structs.is_empty()
            && self.enums.is_empty()
            && self.types.is_empty()
    }
}

/// module name -> its surface. `BTreeMap` throughout for deterministic
/// serialization (SC-005) regardless of source declaration order.
pub type Surface = BTreeMap<String, ModuleSurface>;

/// Reads a `lib.rs`'s top-level `pub mod NAME;` declarations, in source
/// order — the common path both disk-based and `git show`-based extraction
/// share (each reads the source text differently, then calls this).
pub fn public_module_names_from_source(lib_rs_content: &str) -> Vec<String> {
    let file = syn::parse_file(lib_rs_content).unwrap_or_else(|e| panic!("parsing lib.rs: {e}"));
    file.items
        .into_iter()
        .filter_map(|item| match item {
            Item::Mod(m) if matches!(m.vis, Visibility::Public(_)) && m.content.is_none() => {
                Some(m.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

/// Extracts the full public surface of a `hl7pet-core`-shaped crate:
/// `lib_rs_content`'s `pub mod` declarations, each resolved to its own
/// source text via `read_module` (a module name -> its file's full source)
/// and parsed for its top-level public items. Disk- and `git show`-based
/// extraction (for `--against <commit>`) both funnel through this — the
/// only difference is where `read_module` gets its bytes from.
pub fn extract_surface_from(lib_rs_content: &str, mut read_module: impl FnMut(&str) -> String) -> Surface {
    let mut surface = Surface::new();
    for module in public_module_names_from_source(lib_rs_content) {
        let content = read_module(&module);
        let file = syn::parse_file(&content).unwrap_or_else(|e| panic!("parsing module {module}: {e}"));
        let module_surface = extract_module_surface(&file.items);
        if !module_surface.is_empty() {
            surface.insert(module, module_surface);
        }
    }
    surface
}

/// Extracts the full public surface directly from `src_dir` on disk
/// (`src_dir/lib.rs` plus each `pub mod`'s own `<name>.rs`/`<name>/mod.rs`).
pub fn extract_surface(src_dir: &Path) -> Surface {
    let lib_rs = src_dir.join("lib.rs");
    let lib_rs_content =
        fs::read_to_string(&lib_rs).unwrap_or_else(|e| panic!("reading {}: {e}", lib_rs.display()));
    extract_surface_from(&lib_rs_content, |module| {
        let module_path = module_file(src_dir, module);
        fs::read_to_string(&module_path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", module_path.display()))
    })
}

fn module_file(src_dir: &Path, module: &str) -> PathBuf {
    let direct = src_dir.join(format!("{module}.rs"));
    if direct.exists() {
        return direct;
    }
    src_dir.join(module).join("mod.rs")
}

fn extract_module_surface(items: &[Item]) -> ModuleSurface {
    let mut surface = ModuleSurface::default();
    for item in items {
        if is_doc_hidden(item_attrs(item)) {
            continue;
        }
        match item {
            Item::Fn(f) if matches!(f.vis, Visibility::Public(_)) => {
                surface
                    .functions
                    .insert(f.sig.ident.to_string(), normalize_signature(&f.sig));
            }
            Item::Struct(s) if matches!(s.vis, Visibility::Public(_)) => {
                surface.structs.insert(s.ident.to_string(), struct_def(&s.fields));
            }
            Item::Enum(e) if matches!(e.vis, Visibility::Public(_)) => {
                surface.enums.insert(e.ident.to_string(), enum_def(e));
            }
            Item::Type(t) if matches!(t.vis, Visibility::Public(_)) => {
                let ty = &t.ty;
                let rhs = quote!(#ty).to_string();
                surface.types.insert(t.ident.to_string(), normalize_tokens(&rhs));
            }
            _ => {}
        }
    }
    surface
}

fn item_attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Fn(f) => &f.attrs,
        Item::Struct(s) => &s.attrs,
        Item::Enum(e) => &e.attrs,
        Item::Type(t) => &t.attrs,
        _ => &[],
    }
}

fn is_doc_hidden(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("doc") && attr.parse_args::<syn::Ident>().is_ok_and(|id| id == "hidden")
    })
}

fn normalize_signature(sig: &syn::Signature) -> String {
    normalize_tokens(&quote!(#sig).to_string())
}

fn normalize_tokens(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn struct_def(fields: &Fields) -> StructDef {
    let mut def = StructDef::default();
    if let Fields::Named(named) = fields {
        for field in &named.named {
            if !matches!(field.vis, Visibility::Public(_)) {
                continue;
            }
            let name = field.ident.as_ref().unwrap().to_string();
            let ty_tokens = &field.ty;
            let ty = normalize_tokens(&quote!(#ty_tokens).to_string());
            def.fields.insert(name, ty);
        }
    }
    def
}

fn enum_def(e: &syn::ItemEnum) -> EnumDef {
    let mut variants: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            let name = v.ident.to_string();
            match &v.fields {
                Fields::Unit => name,
                Fields::Named(named) => {
                    let fields: Vec<String> = named
                        .named
                        .iter()
                        .map(|f| {
                            let ty = &f.ty;
                            format!(
                                "{}: {}",
                                f.ident.as_ref().unwrap(),
                                normalize_tokens(&quote!(#ty).to_string())
                            )
                        })
                        .collect();
                    format!("{name} {{ {} }}", fields.join(", "))
                }
                Fields::Unnamed(unnamed) => {
                    let fields: Vec<String> = unnamed
                        .unnamed
                        .iter()
                        .map(|f| {
                            let ty = &f.ty;
                            normalize_tokens(&quote!(#ty).to_string())
                        })
                        .collect();
                    format!("{name}({})", fields.join(", "))
                }
            }
        })
        .collect();
    variants.sort();
    EnumDef { variants }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Item> {
        syn::parse_file(src).unwrap().items
    }

    #[test]
    fn extracts_a_public_function_signature() {
        let items = parse("pub fn foo(x: u32) -> bool { x > 0 }");
        let surface = extract_module_surface(&items);
        assert_eq!(surface.functions.get("foo").unwrap(), "fn foo (x : u32) -> bool");
    }

    #[test]
    fn skips_private_items() {
        let items = parse("fn foo() {} struct Bar;");
        let surface = extract_module_surface(&items);
        assert!(surface.is_empty());
    }

    #[test]
    fn skips_doc_hidden_items() {
        let items = parse("#[doc(hidden)] pub fn internal_only() {}");
        let surface = extract_module_surface(&items);
        assert!(surface.is_empty());
    }

    #[test]
    fn extracts_public_struct_fields_only() {
        let items = parse("pub struct S { pub a: u32, b: String }");
        let surface = extract_module_surface(&items);
        let def = surface.structs.get("S").unwrap();
        assert_eq!(def.fields.len(), 1);
        assert_eq!(def.fields.get("a").unwrap(), "u32");
    }

    #[test]
    fn extracts_enum_variants_with_payloads() {
        let items = parse("pub enum E { A, B(u32), C { x: bool } }");
        let surface = extract_module_surface(&items);
        let def = surface.enums.get("E").unwrap();
        assert_eq!(def.variants, vec!["A".to_string(), "B(u32)".to_string(), "C { x: bool }".to_string()]);
    }
}
