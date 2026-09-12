const form = document.getElementById("extract-form");
const resultsEl = document.getElementById("results");

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const formData = new FormData(form);

  let response;
  try {
    response = await fetch("/api/extract", { method: "POST", body: formData });
  } catch (err) {
    render({ status: "path_error", message: "Network error: " + err.message });
    return;
  }

  if (response.status === 413) {
    render(await response.json());
    return;
  }

  render(await response.json());
});

function render(data) {
  // Every submission fully replaces prior results (FR-006) -- no accumulation.
  resultsEl.innerHTML = "";
  resultsEl.className = "";

  switch (data.status) {
    case "results":
      renderResults(data);
      break;
    case "no_results":
      renderEmpty();
      break;
    case "profile_required":
      renderNotice("profile_required", data.message);
      break;
    case "path_error":
    case "scan_error":
    case "profile_error":
    case "too_large":
      renderError(data.status, data.message);
      break;
    default:
      renderError("path_error", "Unexpected response from the server.");
  }
}

function renderResults(data) {
  resultsEl.className = "state-results";

  if (data.hierarchy) {
    const note = document.createElement("p");
    note.className = "note";
    note.textContent = "Line numbers aren't available for hierarchy PATHs yet.";
    resultsEl.appendChild(note);

    const list = document.createElement("ul");
    list.className = "result-list";
    data.results.forEach((value) => {
      const li = document.createElement("li");
      li.className = "result-row";
      const valueEl = document.createElement("span");
      valueEl.className = "result-value";
      valueEl.textContent = value;
      li.appendChild(valueEl);
      list.appendChild(li);
    });
    resultsEl.appendChild(list);
    return;
  }

  const list = document.createElement("ul");
  list.className = "result-list";
  data.results.forEach((entry) => {
    const li = document.createElement("li");
    li.className = "result-row";

    const valueEl = document.createElement("span");
    valueEl.className = "result-value";
    valueEl.textContent = entry.value;

    const lineEl = document.createElement("span");
    lineEl.className = "result-line";
    lineEl.textContent = "line " + entry.line;

    li.appendChild(valueEl);
    li.appendChild(lineEl);
    list.appendChild(li);
  });
  resultsEl.appendChild(list);
}

function renderEmpty() {
  resultsEl.className = "state-empty";
  const p = document.createElement("p");
  p.textContent = "No results — that PATH didn't match anything in this message.";
  resultsEl.appendChild(p);
}

function renderNotice(kind, message) {
  resultsEl.className = "state-notice state-" + kind;
  const p = document.createElement("p");
  p.textContent = message;
  resultsEl.appendChild(p);
}

function renderError(kind, message) {
  resultsEl.className = "state-error state-" + kind;
  const p = document.createElement("p");
  p.textContent = message;
  resultsEl.appendChild(p);
}
