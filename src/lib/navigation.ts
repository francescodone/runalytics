import { icons } from "./icons";
import { fmtDateDMY } from "./ipc";

const nav = document.querySelector<HTMLElement>("#nav");
const main = document.querySelector<HTMLElement>("#app");

if (nav && main) {
  const destinations = document.createElement("div");
  destinations.id = "nav-destinations";
  destinations.className = "nav-destinations";
  const links = [...nav.querySelectorAll<HTMLAnchorElement>(".nav-link")];
  const activeLink = links.find((link) => link.classList.contains("active"));
  const title = activeLink?.textContent?.trim() ?? "Runalytics";
  const descriptions: Record<string, string> = {
    Dashboard: "Load, injury risk and body signals in one place.",
    Plan: "Build a training plan that moves with you.",
    Calendar: "Every session, all in one place.",
    Injuries: "Keep track of how your body feels.",
    Achievements: "See how far you've come.",
    Settings: "Make Runalytics yours.",
  };
  const navIcons: Record<string, string> = {
    "index.html": icons.chart,
    "plan.html": icons.flag,
    "calendar.html": icons.calendar,
    "injury.html": icons.heart,
    "achievements.html": icons.trophy,
    "settings.html": icons.sliders,
  };

  for (const link of links) {
    const symbol = document.createElement("span");
    symbol.className = "nav-icon";
    symbol.innerHTML = navIcons[link.getAttribute("href") ?? ""] ?? icons.chart;
    link.prepend(symbol);
    if (link === activeLink) link.setAttribute("aria-current", "page");
    destinations.append(link);
  }
  // The brand mark mirrors the app icon (scripts/generate_app_icon.py): the
  // brand gradient squircle carrying a white tile with the italic R. Built
  // here so the eight HTML pages keep a plain <span class="brand">.
  const brand = nav.querySelector<HTMLElement>(".brand");
  if (brand) {
    const name = brand.textContent?.trim() ?? "Runalytics";
    brand.textContent = "";
    const mark = document.createElement("span");
    mark.className = "brand-mark";
    mark.setAttribute("aria-hidden", "true");
    const tile = document.createElement("span");
    tile.className = "brand-tile";
    tile.textContent = "R";
    mark.append(tile);
    const label = document.createElement("span");
    label.className = "brand-name";
    label.textContent = name;
    brand.append(mark, label);
  }
  brand?.after(destinations);

  // The footer pins Settings to the bottom of the rail, with the sync chip
  // (mounted by sync.ts) directly above it. On the mobile toolbar the footer
  // dissolves — `display: contents` — and Settings moves back into the
  // dropdown, which is where a secondary destination belongs.
  const footer = document.createElement("div");
  footer.id = "nav-footer";
  const settingsLink = links.find((link) => link.getAttribute("href") === "settings.html");
  const compact = window.matchMedia("(max-width: 900px)");
  const placeSettings = () => {
    if (!settingsLink) return;
    if (compact.matches) destinations.append(settingsLink);
    else footer.append(settingsLink);
  };
  placeSettings();
  nav.append(footer);

  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "nav-toggle secondary icon-btn";
  toggle.innerHTML = icons.chevronRight;
  toggle.setAttribute("aria-controls", destinations.id);
  nav.append(toggle);

  const setOpen = (open: boolean) => {
    nav.dataset.open = String(open);
    toggle.setAttribute("aria-expanded", String(open));
    toggle.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
    toggle.title = open ? "Close navigation" : "Open navigation";
  };
  setOpen(false);
  toggle.addEventListener("click", () => setOpen(nav.dataset.open !== "true"));
  nav.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && nav.dataset.open === "true") {
      setOpen(false);
      toggle.focus();
    }
  });
  document.addEventListener("click", (event) => {
    if (event.target instanceof Node && !nav.contains(event.target)) setOpen(false);
  });
  compact.addEventListener("change", () => {
    if (compact.matches && destinations.contains(document.activeElement)) toggle.focus();
    if (!compact.matches && document.activeElement === toggle) activeLink?.focus();
    placeSettings();
    setOpen(false);
  });

  const header = document.createElement("header");
  header.className = "page-header";
  const heading = document.createElement("h1");
  heading.textContent = title;
  const intro = document.createElement("div");
  intro.className = "page-intro";
  const description = document.createElement("p");
  description.textContent = descriptions[title] ?? "Your running, in focus.";
  intro.append(heading, description);
  header.append(intro);
  if (activeLink?.getAttribute("href") === "index.html") {
    heading.textContent = "Your training";
    const actions = document.createElement("div");
    actions.className = "page-actions";
    const date = document.createElement("time");
    const today = new Date();
    date.dateTime = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    date.textContent = `${today.toLocaleDateString(undefined, {
      weekday: "long",
    })} \u00b7 ${fmtDateDMY(today)}`;
    const planLink = document.createElement("a");
    planLink.className = "header-action";
    planLink.id = "header-plan-cta";
    planLink.href = "plan.html";
    planLink.innerHTML = `${icons.plus}<span>Training plan</span>`;
    actions.append(date, planLink);
    header.append(actions);
  }
  main.prepend(header);
}