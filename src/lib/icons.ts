// Icon set used in place of emoji everywhere in the UI.
//
// Paths are taken from Tabler Icons (https://github.com/tabler/tabler-icons,
// MIT) — outline style, 24x24 grid. They are inlined here (rather than
// imported from the npm package) so each icon renders as a string with
// `stroke: currentColor` at 1em: it inherits the surrounding text colour and
// size, stays crisp, and looks the same across platforms (emoji rendering is
// not).

const svg = (paths: string) =>
  `<svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`;

export const icons = {
  // tabler: device-watch
  watch: svg(
    `<path d="M6 9a3 3 0 0 1 3-3h6a3 3 0 0 1 3 3v6a3 3 0 0 1-3 3H9a3 3 0 0 1-3-3V9"/><path d="M9 18v3h6v-3"/><path d="M9 6V3h6v3"/>`,
  ),
  // tabler: refresh
  sync: svg(
    `<path d="M20 11a8.1 8.1 0 0 0-15.5-2m-.5-4v4h4"/><path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4"/>`,
  ),
  // tabler: calendar
  calendar: svg(
    `<path d="M4 7a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7"/><path d="M16 3v4M8 3v4M4 11h16"/><path d="M11 15h1M12 15v3"/>`,
  ),
  // tabler: flag
  flag: svg(`<path d="M5 5a5 5 0 0 1 7 0a5 5 0 0 0 7 0v9a5 5 0 0 1-7 0a5 5 0 0 0-7 0v-9"/><path d="M5 21v-7"/>`),
  // tabler: check
  check: svg(`<path d="M5 12l5 5l10-10"/>`),
  // tabler: flame
  flame: svg(
    `<path d="M12 10.941c2.333-3.308.167-7.823-1-8.941c0 3.395-2.235 5.299-3.667 6.706c-1.43 1.408-2.333 3.294-2.333 5.588c0 3.704 3.134 6.706 7 6.706c3.866 0 7-3.002 7-6.706c0-1.712-1.232-4.403-2.333-5.588c-2.084 3.353-3.257 3.353-4.667 2.235"/>`,
  ),
  // tabler: target
  target: svg(
    `<path d="M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0"/><path d="M7 12a5 5 0 1 0 10 0a5 5 0 1 0-10 0"/><path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0"/>`,
  ),
  // tabler: lock
  lock: svg(
    `<path d="M5 13a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-6"/><path d="M11 16a1 1 0 1 0 2 0a1 1 0 0 0-2 0"/><path d="M8 11V7a4 4 0 1 1 8 0v4"/>`,
  ),
  // tabler: run
  run: svg(
    `<path d="M11.007 5a2 2 0 1 0 4 0a2 2 0 1 0-4 0"/><path d="M4 17l5 1l.75-1.5"/><path d="M15 21v-4l-4-3l1-6"/><path d="M7 12V9l5-1l3 3l3 1"/>`,
  ),
  // tabler: bike
  bike: svg(
    `<circle cx="5" cy="17" r="3"/><circle cx="19" cy="17" r="3"/><path d="M12 17V9"/><path d="M9 5h2l3 4"/><path d="M5 17l4-8h6"/><path d="M17 17l-2.5-6"/>`,
  ),
  // tabler: dumbbell — the cross-training glyph. A bike would name one sport
  // (cycling) on tiles that also cover swimming, strength and hike days.
  dumbbell: svg(
    `<path d="M4 12h16"/><path d="M4 8v8"/><path d="M7 6v12"/><path d="M17 6v12"/><path d="M20 8v8"/>`,
  ),
  // tabler: chart-bar
  chart: svg(
    `<path d="M3 13a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-6"/><path d="M15 9a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1V9"/><path d="M9 5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1V5"/><path d="M4 20h14"/>`,
  ),
  // tabler: trophy
  trophy: svg(
    `<path d="M8 21h8"/><path d="M12 17v4"/><path d="M7 4h10"/><path d="M17 4v8a5 5 0 0 1-10 0V4"/><path d="M3 9a2 2 0 1 0 4 0a2 2 0 1 0-4 0"/><path d="M17 9a2 2 0 1 0 4 0a2 2 0 1 0-4 0"/>`,
  ),
  // tabler: sunrise
  sunrise: svg(
    `<path d="M3 17h1m16 0h1m-15.4-6.4l.7.7m12.1-.7l-.7.7m-9.7 5.7a4 4 0 0 1 8 0"/><path d="M3 21h18"/><path d="M12 9V3l3 3m-6 0l3-3"/>`,
  ),
  // tabler: moon
  moon: svg(`<path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1-8.313-12.454l.008.008"/>`),
  // tabler: bolt
  bolt: svg(`<path d="M13 3l0 7l6 0l-8 11l0-7l-6 0l8-11"/>`),
  // tabler: heart
  heart: svg(`<path d="M19.5 12.572l-7.5 7.428l-7.5-7.428a5 5 0 1 1 7.5-6.566a5 5 0 1 1 7.5 6.572"/>`),
  // tabler: mountain
  mountain: svg(
    `<path d="M3 20h18l-6.921-14.612a2.3 2.3 0 0 0-4.158 0L3 20"/><path d="M7.5 11l2 2.5l2.5-2.5l2 3l2.5-2"/>`,
  ),
  // tabler: clock
  clock: svg(`<path d="M3 12a9 9 0 1 0 18 0a9 9 0 0 0-18 0"/><path d="M12 7v5l3 3"/>`),
  // tabler: star
  star: svg(
    `<path d="M12 17.75l-6.172 3.245l1.179-6.873l-5-4.867l6.9-1l3.086-6.253l3.086 6.253l6.9 1l-5 4.867l1.179 6.873l-6.158-3.245"/>`,
  ),
  // tabler: medal-2
  medal: svg(
    `<path d="M9 3h6l3 7l-6 2l-6-2l3-7"/><path d="M12 12l-3-9"/><path d="M15 11l-3-8"/><path d="M12 19.5l-3 1.5l.5-3.5l-2-2l3-.5l1.5-3l1.5 3l3 .5l-2 2l.5 3.5l-3-1.5"/>`,
  ),
  // tabler: map
  map: svg(`<path d="M3 7l6-3l6 3l6-3v13l-6 3l-6-3l-6 3V7"/><path d="M9 4v13"/><path d="M15 7v13"/>`),
  // tabler: palette — the Appearance category in Settings.
  palette: svg(
    `<path d="M12 21a9 9 0 1 1 0-18c4.97 0 9 3.582 9 8 0 1.06-.84 2-2 2h-2a3 3 0 0 0-3 3c0 .797.365 1.508.936 1.973.4.326.664.812.554 1.337-.12.56-.64.69-1.49.69"/><path d="M7.5 11.5a.5.5 0 1 0 0-1 .5.5 0 0 0 0 1"/><path d="M11 8a.5.5 0 1 0 0-1 .5.5 0 0 0 0 1"/><path d="M15.5 9.5a.5.5 0 1 0 0-1 .5.5 0 0 0 0 1"/>`,
  ),
  // tabler: adjust-similar — the Training zones category (editable boundaries).
  sliders: svg(
    `<path d="M4 6h8M17 6h3M4 12h3M12 12h8M4 18h8M17 18h3"/><path d="M14 6a2 2 0 1 1-4 0a2 2 0 0 1 4 0"/><path d="M9 12a2 2 0 1 1-4 0a2 2 0 0 1 4 0"/><path d="M16 18a2 2 0 1 1-4 0a2 2 0 0 1 4 0"/>`,
  ),
  // tabler: chevron-left / chevron-right — the carousel paging affordance.
  chevronLeft: svg(`<path d="M15 6l-6 6l6 6"/>`),
  chevronRight: svg(`<path d="M9 6l6 6l-6 6"/>`),
  // Map controls. The library's own zoom/compass icons are baked into CSS
  // background images with a hardcoded colour, so they can't follow the app's
  // theme — these replace them and inherit currentColor like every other icon.
  // tabler: plus
  plus: svg(`<path d="M12 5l0 14"/><path d="M5 12l14 0"/>`),
  // tabler: minus
  minus: svg(`<path d="M5 12l14 0"/>`),
  // tabler: frame — the map's own extent, not a generic expand arrow.
  fit: svg(
    `<path d="M4 7l16 0"/><path d="M4 17l16 0"/><path d="M7 4l0 16"/><path d="M17 4l0 16"/>`,
  ),
  // tabler: focus-2 — centre on something.
  locate: svg(
    `<path d="M11.5 12a.5 .5 0 1 0 1 0a.5 .5 0 1 0 -1 0" fill="currentColor"/><path d="M5 12a7 7 0 1 0 14 0a7 7 0 1 0 -14 0"/><path d="M12 3l0 2"/><path d="M3 12l2 0"/><path d="M12 19l0 2"/><path d="M19 12l2 0"/>`,
  ),
  // tabler: arrow-left / arrow-right — stepping a month back or forward. A
  // chevron means "page sideways within a row" (the carousels use those); an
  // arrow means "go to the previous/next thing in time", which is what a
  // calendar does. The HTML entities these replace (&larr;) rendered in
  // whatever the platform font drew, which is why they matched nothing else.
  arrowLeft: svg(`<path d="M12 5l-7 7l7 7"/><path d="M5 12h14"/>`),
  arrowRight: svg(`<path d="M12 5l7 7l-7 7"/><path d="M5 12h14"/>`),
  // Half the disc filled: the two ends of the day in one mark. Used for
  // "Night & day" (a run before 6:00 and one after 21:00), which a globe
  // described as "worldwide" — nothing to do with hours.
  contrast: svg(
    `<path d="M12 3a9 9 0 1 0 0 18z" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="9"/>`,
  ),
  // tabler: calendar with a grid of days — a whole year of months, for
  // "active in 12 different months". `calendar` marks one day (Monday club);
  // this one marks the twelve.
  calendarYear: svg(
    `<path d="M4 7a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7"/><path d="M16 3v4M8 3v4M4 11h16"/><path d="M8 14.5h.01M12 14.5h.01M16 14.5h.01M8 17.5h.01M12 17.5h.01M16 17.5h.01"/>`,
  ),
  // tabler: bed — a deliberate break from training. The moon was doing double
  // duty for "night run" and "14-day rest"; night keeps the moon, and rest
  // gets the thing you actually do instead of running.
  bed: svg(
    `<path d="M3 7v11"/><path d="M3 16h18v4"/><path d="M21 16v-5a2 2 0 0 0-2-2H3"/><path d="M6 11v-.5a1.5 1.5 0 0 1 1.5-1.5h3A1.5 1.5 0 0 1 12 10.5V11"/>`,
  ),
  // tabler: stack-2 — a pile of sessions, for "N sessions synced". A bar chart
  // plots a quantity; these badges count events, and stacked cards say "many
  // sessions" without pretending to be a graph.
  stack: svg(
    `<path d="M12 3l8 4l-8 4l-8-4l8-4"/><path d="M4 12l8 4l8-4"/><path d="M4 17l8 4l8-4"/>`,
  ),
  // tabler: trending-up — recent form, which is a pace trend over a window.
  // The bar-chart glyph was borrowed for this and for the session counters;
  // a rising line is what "Sharp month" actually measures.
  trend: svg(`<path d="M3 17l6-6l4 4l7-7"/><path d="M14 8h6v6"/>`),
  // tabler: repeat — returning after a break. `sync` means "data moved to the
  // watch"; a comeback is the runner repeating the habit, so the loop reads as
  // the return rather than a transfer.
  repeat: svg(
    `<path d="M4 12a8 8 0 0 1 13.66-5.66L20 8"/><path d="M20 4v4h-4"/><path d="M20 12a8 8 0 0 1-13.66 5.66L4 16"/><path d="M4 20v-4h4"/>`,
  ),
  // tabler: gift — a run on a festive day. `flag` means race day everywhere
  // else in the app, and December 25th is not a race.
  gift: svg(
    `<path d="M15.945 8.75c2.125-1.879 4.566-1.879 6.055 0v11.25h-18v-11.25c1.489-1.879 3.93-1.879 6.055 0"/><path d="M3 13h18"/><path d="M12 20v-12.5"/><path d="M12 8h-3.5a2.5 2.5 0 0 1 0-5c3 0 3.5 2 3.5 5"/><path d="M12 8h3.5a2.5 2.5 0 0 0 0-5c-3 0-3.5 2-3.5 5"/>`,
  ),
  // tabler: alert-triangle — Friday the 13th, the app's one superstition.
  // Reads as "unlucky" where a flag reads as "race".
  alert: svg(
    `<path d="M10.46 3.71a1.8 1.8 0 0 1 3.08 0l8.55 14.09a1.8 1.8 0 0 1-1.54 2.7h-17.1a1.8 1.8 0 0 1-1.54-2.7z"/><path d="M12 9v4"/><path d="M12 17h.01"/>`,
  ),
  // tabler: sparkles — the assistant category. A heart said "favourite"; this
  // pane configures the language model that answers questions.
  sparkles: svg(
    `<path d="M14 3l1.337 3.464l1 .733l3.663.803l-2.225 2.56l.237 1.241l1.325 3.2l-3.53-.357l-1.037.675l-1.837 3.398l-.87-3.597l-.926-.823l-3.564-1.237l2.878-2.276l.237-1.241l-.538-3.774l3.408 1.888h1.155z"/><path d="M5 3l.618 1.725l.426.426l1.956.585l-1.284 1.32l-.096.844l.48 1.92l-1.908-.456l-.744.384l-1.2 1.824l-.372-1.992l-.552-.576l-1.8-.768l1.416-1.248l.096-.844l-.264-1.824l1.728.888z"/>`,
  ),
  // tabler: info-circle — the About category. A star means "favourite" or
  // "special"; this pane is version and credit information.
  info: svg(
    `<path d="M3 12a9 9 0 1 0 18 0a9 9 0 0 0-18 0"/><path d="M12 9v4"/><path d="M12 16h.01"/>`,
  ),  // tabler: rocket — the Setup category. Getting started is the one thing in
  // the app you do once, and a launch is the only metaphor that says that.
  rocket: svg(
    `<path d="M4 13a8 8 0 0 1 7 7a6 6 0 0 0 3 -5a9 9 0 0 0 6 -8a3 3 0 0 0 -3 -3a9 9 0 0 0 -8 6a6 6 0 0 0 -5 3"/><path d="M7 14a6 -6 0 0 0 -3 4a3 -3 0 0 1 0 -4a1.5 1.5 0 0 1 3 -1"/><circle cx="15" cy="9" r="1"/>`,
  ),};

/**
 * Fill every `<span data-icon="…">` in the document with its glyph. The icons
 * live in this one module so they inherit `currentColor` and stay identical to
 * the ones the JS-rendered panels use; pasting SVG into the HTML would give the
 * same glyph two sources, and HTML entities (`&larr;`) render as whatever the
 * platform's font happens to draw — which is why the calendar's month arrows
 * looked nothing like the rest of the icon set.
 */
export function hydrateIcons(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-icon]").forEach((span) => {
    const name = span.dataset.icon as keyof typeof icons | undefined;
    if (name && name in icons) span.innerHTML = icons[name];
  });
}
