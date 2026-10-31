import { icons } from "./icons";
import { parsePace } from "./ipc";

export function initPlanWizard(form: HTMLFormElement): void {
  const grid = form.querySelector<HTMLElement>(".plan-grid");
  if (!grid) return;
  const fields = [...grid.children];
  const groups = [[0], [1, 2, 3], [4, 5], [6, 7, 8], []];
  const titles = ["What are you training for?", "Choose your target", "Your running baseline", "Make it fit your week", "Your plan, at a glance"];
  const progress = document.createElement("div");
  progress.className = "wizard-progress";
  progress.setAttribute("aria-live", "polite");
  const heading = document.createElement("h3");
  heading.className = "wizard-title";
  heading.tabIndex = -1;
  const steps = groups.map((indices, index) => {
    const step = document.createElement("div");
    step.className = "wizard-step";
    step.dataset.step = String(index);
    for (const position of indices) if (fields[position]) step.append(fields[position]);
    return step;
  });
  const explainer = form.querySelector(".threshold-explainer");
  if (explainer) steps[2].append(explainer);
  const preview = form.querySelector("#plan-preview");
  const submit = form.querySelector(".plan-submit");
  if (preview) steps[4].append(preview);
  if (submit) steps[4].append(submit);
  grid.replaceChildren(...steps);
  form.prepend(progress, heading);
  const controls = document.createElement("div");
  controls.className = "wizard-controls";
  const back = document.createElement("button");
  back.type = "button";
  back.className = "secondary";
  back.innerHTML = `${icons.arrowLeft}<span>Back</span>`;
  const next = document.createElement("button");
  next.type = "button";
  next.innerHTML = `<span>Continue</span>${icons.arrowRight}`;
  controls.append(back, next);
  form.append(controls);
  let current = 0;
  const show = (index: number, focus = true) => {
    current = index;
    steps.forEach((step, position) => { step.hidden = position !== index; });
    progress.textContent = `Step ${index + 1} of ${steps.length}`;
    heading.textContent = titles[index];
    back.hidden = index === 0;
    next.hidden = index === steps.length - 1;
    if (focus) heading.focus();
  };
  next.addEventListener("click", () => {
    const inputs = [...steps[current].querySelectorAll<HTMLInputElement>("input")];
    for (const input of inputs) {
      if (input.closest<HTMLElement>("[hidden]")) continue;
      input.setCustomValidity(input.id === "plan-threshold" && parsePace(input.value) == null ? "Enter a pace such as 4:45." : "");
      if (!input.reportValidity()) return;
    }
    show(Math.min(steps.length - 1, current + 1));
  });
  form.addEventListener("input", (event) => {
    if (event.target instanceof HTMLInputElement) event.target.setCustomValidity("");
  });
  form.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && current < steps.length - 1 && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      next.click();
    }
  });
  form.addEventListener("submit", (event) => {
    if (current !== steps.length - 1) {
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    const invalid = form.querySelector<HTMLInputElement>('input[aria-invalid="true"]');
    if (invalid) {
      const step = steps.findIndex((candidate) => candidate.contains(invalid));
      if (step >= 0) show(step);
    }
  }, true);
  back.addEventListener("click", () => show(Math.max(0, current - 1)));
  show(0, false);
}