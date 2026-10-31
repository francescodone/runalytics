import { icons } from "./icons";
import appIcon from "../../src-tauri/icons/128x128@2x.png";
import "../styles/redesign.css";

const nav = document.querySelector<HTMLElement>("#nav");
const main = document.querySelector<HTMLElement>("#app");

if (nav && main) {
  const destinations = document.createElement("div");
  destinations.id = "nav-destinations";
  destinations.className = "nav-destinations";
  const links = [...nav.querySelectorAll<HTMLAnchorElement>(".nav-link")];
  const activeLink = links.find((link) => link.classList.contains("active"));
  const title = activeLink?.textContent?.trim() ?? "Runalytics";
  document.body.dataset.page = activeLink?.getAttribute("href")?.replace(".html", "") ?? "support";
  const labels: Record<string, string> = {
    "index.html": "Today", "plan.html": "Plan", "calendar.html": "Calendar",
    "achievements.html": "Progress", "injury.html": "Injuries", "support.html": "Support", "settings.html": "Settings",
  };
  for (const link of links) {
    const href = link.getAttribute("href") ?? "";
    if (labels[href]) link.textContent = labels[href];
    if (!["index.html", "plan.html", "calendar.html", "achievements.html"].includes(href)) link.classList.add("nav-secondary");
  }
  if (!links.some((link) => link.getAttribute("href") === "support.html")) {
    const support = document.createElement("a");
    support.href = "support.html";
    support.className = "nav-link";
    support.textContent = "Support";
    links.push(support);
  }
  const navIcons: Record<string, string> = {
    "index.html": icons.sunrise,
    "plan.html": icons.flag,
    "calendar.html": icons.calendar,
    "injury.html": icons.heart,
    "achievements.html": icons.chart,
    "support.html": icons.info,
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
  const brand = nav.querySelector<HTMLElement>(".brand");
  if (brand) {
    const name = brand.textContent?.trim() ?? "Runalytics";
    brand.textContent = "";
    const mark = document.createElement("img");
    mark.className = "brand-mark";
    mark.src = appIcon;
    mark.alt = "";
    mark.width = 32;
    mark.height = 32;
    mark.setAttribute("aria-hidden", "true");
    const label = document.createElement("span");
    label.className = "brand-name";
    label.textContent = name;
    brand.append(mark, label);
  }
  brand?.after(destinations);

  const footer = document.createElement("details");
  footer.id = "nav-footer";
  const more = document.createElement("summary");
  more.className = "nav-more";
  more.innerHTML = `<span class="nav-icon">${icons.sliders}</span><span>More</span>`;
  const utilities = document.createElement("div");
  utilities.className = "nav-utilities";
  for (const link of links.filter((link) => link.classList.contains("nav-secondary") || link.getAttribute("href") === "support.html")) utilities.append(link);
  footer.append(more, utilities);
  const mobile = window.matchMedia("(max-width: 1023px)");
  const arrangeUtilities = () => {
    footer.open = !mobile.matches;
  };
  arrangeUtilities();
  mobile.addEventListener("change", arrangeUtilities);
  if (activeLink && utilities.contains(activeLink)) more.setAttribute("aria-current", "page");
  document.addEventListener("click", (event) => {
    if (mobile.matches && !footer.contains(event.target as Node)) footer.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && mobile.matches && footer.open) {
      footer.open = false;
      more.focus();
    }
  });
  nav.append(footer);

  const header = document.createElement("header");
  header.className = "page-header";
  const heading = document.createElement("h1");
  heading.textContent = title;
  const intro = document.createElement("div");
  intro.className = "page-intro";
  heading.textContent = labels[activeLink?.getAttribute("href") ?? ""] ?? title;
  intro.append(heading);
  header.append(intro);
  main.prepend(header);
}