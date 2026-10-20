import { icons } from "./icons";

const nav = document.querySelector<HTMLElement>("#nav");
const main = document.querySelector<HTMLElement>("#app");

if (nav && main) {
  const destinations = document.createElement("div");
  destinations.id = "nav-destinations";
  destinations.className = "nav-destinations";
  const links = [...nav.querySelectorAll<HTMLAnchorElement>(".nav-link")];
  const activeLink = links.find((link) => link.classList.contains("active"));
  const title = activeLink?.textContent?.trim() ?? "Runalytics";
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
  nav.querySelector(".brand")?.after(destinations);

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
  const compact = window.matchMedia("(max-width: 900px)");
  compact.addEventListener("change", () => {
    if (compact.matches && destinations.contains(document.activeElement)) toggle.focus();
    if (!compact.matches && document.activeElement === toggle) activeLink?.focus();
    setOpen(false);
  });

  const header = document.createElement("header");
  header.className = "page-header";
  const heading = document.createElement("h1");
  heading.textContent = title;
  header.append(heading);
  if (activeLink?.getAttribute("href") === "index.html") {
    const date = document.createElement("time");
    const today = new Date();
    date.dateTime = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    date.textContent = today.toLocaleDateString(undefined, {
      weekday: "long", month: "long", day: "numeric",
    });
    header.append(date);
  }
  main.prepend(header);
}