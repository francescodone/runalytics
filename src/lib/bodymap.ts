export const BODY_REGIONS: Record<string, string> = {
  neck: "Neck",
  "shoulder.left": "Left shoulder",
  "shoulder.right": "Right shoulder",
  chest: "Chest",
  "chest.left": "Left pec",
  "chest.right": "Right pec",
  "biceps.left": "Left biceps",
  "biceps.right": "Right biceps",
  "triceps.left": "Left triceps",
  "triceps.right": "Right triceps",
  "forearm_wrist.left": "Left forearm / wrist",
  "forearm_wrist.right": "Right forearm / wrist",
  abs: "Core / abs",
  "obliques.left": "Left obliques",
  "obliques.right": "Right obliques",
  upper_back: "Upper back / traps",
  "lat.left": "Left lat",
  "lat.right": "Right lat",
  lower_back: "Lower back",
  "lower_back.left": "Left lower back",
  "lower_back.right": "Right lower back",
  "glute_hip.left": "Left glute / hip",
  "glute_hip.right": "Right glute / hip",
  "hip_flexor.left": "Left hip flexor",
  "hip_flexor.right": "Right hip flexor",
  "quad.left": "Left quad",
  "quad.right": "Right quad",
  "it_band.left": "Left IT band",
  "it_band.right": "Right IT band",
  "hamstring.left": "Left hamstring",
  "hamstring.right": "Right hamstring",
  "knee.left": "Left knee",
  "knee.right": "Right knee",
  "shin.left": "Left shin",
  "shin.right": "Right shin",
  "calf.left": "Left calf",
  "calf.right": "Right calf",
  "achilles.left": "Left Achilles",
  "achilles.right": "Right Achilles",
  "ankle_foot.left": "Left ankle / foot",
  "ankle_foot.right": "Right ankle / foot",
  other: "Other",
};

const SPOTS = new Set([
  "neck", "knee.left", "knee.right", "ankle_foot.left", "ankle_foot.right",
  "achilles.left", "achilles.right", "it_band.left", "it_band.right",
  "hip_flexor.left", "hip_flexor.right",
]);

export function regionLabel(key: string): string {
  const [base, side] = key.split(".");
  const word = base === "chest" ? "chest" : base.replace(/_/g, " ");
  return side ? `${side} ${word}` : word;
}

interface Shape {
  key: string;
  el: string;
}

function mirror(el: string): string {
  return el.replace(/([MLCQ])([^MLCQZz]*)/g, (_match, command: string, coordinates: string) => {
    const flipped = coordinates.trim().split(/[\s,]+/).map(Number)
      .map((value, index) => index % 2 === 0 ? 852 - value : value).join(" ");
    return `${command}${flipped}`;
  });
}

function pair(key: string, el: string, front = false): Shape[] {
  return [
    { key: `${key}.${front ? "right" : "left"}`, el },
    { key: `${key}.${front ? "left" : "right"}`, el: mirror(el) },
  ];
}

const HEAD = `<path d="M426,24 C407,24 395,39 397,61 C398,79 401,96 408,103 C417,112 435,112 444,103 C451,96 454,79 455,61 C457,39 445,24 426,24 Z"/>`;
const UPPER_OUTLINE = `M405,96 C406,108 408,119 399,125 C389,133 379,137 367,140 C344,146 338,164 339,196 C339,213 333,239 329,253 C314,284 315,320 308,367 L305,393 C303,401 310,422 316,428 C322,433 326,432 325,423 C325,413 316,409 315,400 L318,391 C320,394 319,404 323,407 C329,410 327,397 326,387 C329,376 321,371 323,362 C330,337 344,309 351,287 C357,270 359,242 363,225 C367,209 371,197 367,181`;
const FRONT_LEG_OUTLINE = `M367,224 C371,246 376,266 372,293 C368,322 361,350 360,380 C358,428 365,468 368,508 C370,533 363,555 366,583 C368,612 376,641 378,660 C378,669 375,676 377,683 C374,694 368,699 364,708 C362,713 362,718 363,721 C368,724 377,725 386,725 C394,729 398,719 397,711 C395,702 399,692 398,687 C395,680 400,676 398,668 C393,644 399,617 403,588 C408,564 401,548 405,531 C411,512 412,486 415,458 C418,432 421,409 422,385 Q426,381 430,385`;
const BACK_LEG_OUTLINE = `M367,224 C371,247 381,268 376,294 C373,316 364,346 361,377 C353,417 361,447 367,484 C372,514 365,539 362,560 C357,581 359,601 362,619 L371,662 C374,679 370,690 371,701 C358,704 358,712 367,717 C375,724 380,728 390,724 C396,722 393,709 394,701 C398,690 392,683 392,672 C393,650 399,625 402,603 C406,582 399,568 402,551 C409,529 411,513 412,492 C415,465 420,436 422,416 L423,398 Q426,389 429,398`;

function outline(back: boolean): string {
  const leg = back ? BACK_LEG_OUTLINE : FRONT_LEG_OUTLINE;
  return `${HEAD}<path d="${UPPER_OUTLINE}"/><path d="${mirror(UPPER_OUTLINE)}"/><path d="${leg}"/><path d="${mirror(leg)}"/>`;
}

const SHOULDER = `<path d="M383,146 C363,145 349,156 346,173 C341,193 345,191 361,180 C369,175 365,165 374,160 C379,157 396,151 386,147 Z"/>`;
const ARM = `<path d="M353,192 C344,194 342,202 342,214 C340,226 334,241 339,251 C347,268 353,246 357,233 C361,220 366,206 361,197 C359,193 357,191 353,192 Z"/>`;
const FOREARM = `<path d="M336,263 C324,260 320,279 320,298 L318,326 C316,341 324,335 329,325 C339,309 349,283 347,273 C345,266 341,262 336,263 Z"/>`;
const NECK: Shape = { key: "neck", el: `<path d="M409,111 Q426,119 443,111 L444,132 Q426,143 408,132 Z"/>` };
const KNEE = `<path d="M373,514 Q388,507 400,516 L400,543 Q386,550 371,542 Z"/>`;
const FOOT = `<path d="M380,653 L394,653 L396,691 L394,718 Q382,727 365,719 Q363,711 376,688 Z"/>`;

const FRONT_SHAPES: Shape[] = [
  NECK,
  ...pair("shoulder", SHOULDER, true),
  ...pair("chest", `<path d="M412,155 C394,152 378,158 374,172 C371,182 376,202 383,209 C390,216 407,213 418,209 C425,207 424,198 424,188 L424,171 C424,160 421,155 412,155 Z"/>`, true),
  ...pair("biceps", ARM, true),
  ...pair("forearm_wrist", FOREARM, true),
  { key: "abs", el: `<path d="M422,216 C414,213 400,218 399,225 L398,260 C397,270 405,267 419,265 C423,264 424,262 424,255 L424,222 Q424,217 422,216 Z"/>
    ${mirror(`<path d="M422,216 C414,213 400,218 399,225 L398,260 C397,270 405,267 419,265 C423,264 424,262 424,255 L424,222 Q424,217 422,216 Z"/>`)}
    <path d="M410,269 C399,269 398,273 398,282 C398,294 398,300 410,300 C422,300 424,298 424,286 C424,274 423,270 410,269 Z"/>
    ${mirror(`<path d="M410,269 C399,269 398,273 398,282 C398,294 398,300 410,300 C422,300 424,298 424,286 C424,274 423,270 410,269 Z"/>`)}
    <path d="M403,303 C397,303 400,316 402,328 L413,364 C416,373 424,372 424,363 L424,313 C424,304 418,302 403,303 Z"/>
    ${mirror(`<path d="M403,303 C397,303 400,316 402,328 L413,364 C416,373 424,372 424,363 L424,313 C424,304 418,302 403,303 Z"/>`)}` },
  ...pair("obliques", `<path d="M381,221 C376,223 378,239 378,253 C379,273 381,289 389,293 C395,298 394,287 394,276 L394,237 C394,226 389,220 381,221 Z"/>
    <path d="M380,295 C375,301 377,313 381,320 L392,337 C400,346 399,332 396,320 C393,309 387,301 380,295 Z"/>`, true),
  ...pair("hip_flexor", `<path d="M381,338 Q391,343 401,357 L410,376 L397,382 Q388,363 379,355 Z"/>`, true),
  ...pair("quad", `<path d="M371,348 C362,369 363,410 366,442 L373,489 C375,500 384,496 382,485 C376,455 373,425 376,397 C378,372 384,351 375,348 Q373,346 371,348 Z"/>
    <path d="M385,362 C377,382 378,405 380,427 C383,451 383,474 384,494 C384,509 395,511 397,492 C401,469 402,451 397,431 C391,410 401,388 393,371 Q389,360 385,362 Z"/>
    <path d="M401,376 C398,392 402,405 401,419 C399,435 408,445 404,466 L399,504 C398,515 404,506 406,491 C411,466 415,438 416,410 C416,392 411,381 401,376 Z"/>`, true),
  ...pair("it_band", `<path d="M359,352 L366,349 Q359,409 369,467 L374,501 L367,502 Q353,424 359,352 Z"/>`, true),
  ...pair("knee", KNEE, true),
  ...pair("shin", `<path d="M373,550 C365,555 369,588 371,608 L378,639 C380,647 380,635 380,620 L379,575 C379,560 377,547 373,550 Z"/>
    <path d="M399,550 C389,562 390,588 391,609 L391,642 C390,650 394,647 395,637 L401,594 C404,574 404,548 399,550 Z"/>`, true),
  ...pair("ankle_foot", FOOT, true),
];

const BACK_SHAPES: Shape[] = [
  NECK,
  { key: "upper_back", el: `<path d="M426,126 C419,124 415,132 404,137 L390,145 C386,149 400,151 405,158 C414,179 408,224 409,243 C409,256 413,256 426,254 C439,256 443,256 443,243 C444,224 438,179 447,158 C452,151 466,149 462,145 L448,137 C437,132 433,124 426,126 Z"/>` },
  ...pair("shoulder", SHOULDER),
  ...pair("lat", `<path d="M393,157 C374,157 373,170 369,180 C365,189 372,192 372,205 C374,224 369,236 374,255 C380,276 377,291 375,310 C374,317 377,315 383,311 L395,302 C397,300 388,296 391,287 L400,265 C401,242 405,215 404,193 C403,173 402,157 393,157 Z"/>`),
  ...pair("triceps", ARM),
  ...pair("forearm_wrist", FOREARM),
  ...pair("lower_back", `<path d="M423,262 L409,265 C404,271 401,281 400,289 C400,300 408,302 414,305 L423,311 L425,306 L425,267 Z"/>`),
  ...pair("glute_hip", `<path d="M417,317 C405,309 387,315 379,329 C369,345 370,351 378,363 C387,375 402,386 415,382 C424,379 423,364 423,348 C423,334 423,324 417,317 Z"/>`),
  ...pair("hamstring", `<path d="M387,390 C374,391 365,400 365,416 C365,438 373,465 375,494 C375,506 382,514 391,511 C402,509 402,500 403,487 L412,436 C417,415 418,405 407,396 C401,391 395,389 387,390 Z"/>`),
  ...pair("knee", `<path d="M375,516 Q387,520 407,516 L400,552 Q386,559 371,552 Z"/>`),
  ...pair("calf", `<path d="M381,562 C368,559 362,576 363,594 C363,612 370,628 374,648 C376,661 384,663 387,651 C389,633 392,619 396,606 C400,585 395,566 381,562 Z"/>`),
  ...pair("achilles", `<path d="M377,660 Q384,665 390,658 L390,683 L387,699 L376,699 L378,679 Z"/>`),
  ...pair("ankle_foot", `<path d="M374,702 L391,700 L392,719 Q387,730 373,719 Q356,710 374,702 Z"/>`),
];

function regionGroup(shape: Shape): string {
  const spot = SPOTS.has(shape.key) ? " spot" : "";
  return `<g class="bodymap-region${spot}" data-region="${shape.key}" tabindex="0" role="button" aria-pressed="false" aria-label="${BODY_REGIONS[shape.key]}"><title>${BODY_REGIONS[shape.key]}</title>${shape.el}</g>`;
}

const BODY_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="285 0 555 752" class="bodymap-svg" role="group" aria-label="Muscle map - click where it hurts">
  <defs><radialGradient id="bodymap-heat" cx="48%" cy="48%" r="65%"><stop offset="0" stop-color="#6ee7b7"/><stop offset="0.55" stop-color="#34d399"/><stop offset="1" stop-color="#059669"/></radialGradient></defs>
  <g data-view="front"><g class="bodymap-deco-outline">${outline(false)}</g>${FRONT_SHAPES.map(regionGroup).join("")}</g>
  <g data-view="back" transform="translate(273 0)"><g class="bodymap-deco-outline">${outline(true)}</g>${BACK_SHAPES.map(regionGroup).join("")}</g>
  <text x="426" y="748" class="bodymap-caption">Front</text><text x="699" y="748" class="bodymap-caption">Back</text>
</svg>`;

export interface BodyMapOptions {
  selected: string | null;
  tints: Map<string, number>;
  onSelect: (key: string) => void;
}

export function renderBodyMap(container: HTMLElement, opts: BodyMapOptions): void {
  container.innerHTML = BODY_SVG;
  const svg = container.querySelector("svg");
  if (!svg) return;
  for (const group of svg.querySelectorAll<SVGGElement>("g.bodymap-region")) {
    const key = group.dataset.region as string;
    const tint = opts.tints.get(key);
    if (tint != null) {
      group.style.setProperty("--tint", String(Math.max(0.15, Math.min(1, tint / 10))));
      group.classList.add("tinted");
    }
    if (key === opts.selected) {
      group.classList.add("selected");
      group.setAttribute("aria-pressed", "true");
    }
    const pick = () => opts.onSelect(key);
    group.addEventListener("click", pick);
    group.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        pick();
      }
    });
  }
}