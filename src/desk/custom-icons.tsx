/**
 * Tabler vector definitions: @tabler/icons-react v3.50.0 (MIT).
 * Copyright (c) 2020-2026 Paweł Kuna
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

import type { CSSProperties, ReactElement, ReactNode } from 'react';

export type CustomIconName =
  | 'run'
  | 'close'
  | 'bike'
  | 'strength'
  | 'flame'
  | 'mountain'
  | 'rest'
  | 'palette'
  | 'language'
  | 'disconnect'
  | 'qr'
  | 'contrast'
  | 'finishFlag'
  | 'strava'
  | 'brand';

const joint = (x: number, y: number): CSSProperties => ({
  transformBox: 'view-box',
  transformOrigin: `${x}px ${y}px`,
});

const trace = { pathLength: 1, strokeDasharray: '1 1', strokeDashoffset: 0 };

function artwork(name: CustomIconName): ReactNode {
  switch (name) {
    case 'run':
      return <g data-icon-part="run-body">
        <path d="M11.007 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" />
        <g data-icon-part="run-leg-back" style={{ ...joint(11, 14), transform: 'rotate(90deg)' }}>
          <path d="M11 14l3.6 2.7" />
          <g data-icon-part="run-knee-back" style={joint(14.6, 16.7)}><path d="M14.6 16.7v4" /></g>
        </g>
        <g data-icon-part="run-leg-front" style={joint(11, 14)}>
          <path d="M11 14l3.6 2.7" />
          <g data-icon-part="run-knee-front" style={joint(14.6, 16.7)}><path d="M14.6 16.7v4" /></g>
        </g>
        <path d="M11 14l1 -6" />
        <g data-icon-part="run-arm-back" style={joint(12, 8)}>
          <path d="M12 8l2.5 2.7" />
          <g data-icon-part="run-elbow-back" style={joint(14.5, 10.7)}><path d="M14.5 10.7l2.8 -1.4" /></g>
        </g>
        <g data-icon-part="run-arm-front" style={{ ...joint(12, 8), transform: 'rotate(90deg)' }}>
          <path d="M12 8l2.5 2.7" />
          <g data-icon-part="run-elbow-front" style={{ ...joint(14.5, 10.7), transform: 'rotate(-90deg)' }}><path d="M14.5 10.7l2.8 -1.4" /></g>
        </g>
      </g>;
    case 'close':
      return <g data-icon-part="close-cross" style={joint(12, 12)}>
        <path d="M6 6l12 12M18 6L6 18" />
      </g>;
    case 'bike':
      return <>
        <path data-icon-part="bike-wheel-back" {...trace} d="M2 18a3 3 0 1 0 6 0a3 3 0 0 0 -6 0" />
        <path data-icon-part="bike-wheel-front" {...trace} d="M16 18a3 3 0 1 0 6 0a3 3 0 0 0 -6 0" />
        <path d="M12 19v-4l-3 -3l5 -4l2 3h3" />
        <path d="M13.007 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" />
      </>;
    case 'strength':
      return <g data-icon-part="strength-lift" style={joint(12, 12)}>
        <path d="M2 12h1" />
        <path d="M6 8h-2a1 1 0 0 0 -1 1v6a1 1 0 0 0 1 1h2" />
        <path d="M6 7v10a1 1 0 0 0 1 1h1a1 1 0 0 0 1 -1v-10a1 1 0 0 0 -1 -1h-1a1 1 0 0 0 -1 1" />
        <path d="M9 12h6" />
        <path d="M15 7v10a1 1 0 0 0 1 1h1a1 1 0 0 0 1 -1v-10a1 1 0 0 0 -1 -1h-1a1 1 0 0 0 -1 1" />
        <path d="M18 8h2a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-2" />
        <path d="M22 12h-1" />
      </g>;
    case 'flame':
      // The source has one contour: only its inward tongue edge sways.
      return <>
        <path data-icon-part="flame-inner" style={joint(12, 10.941)} d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941" />
        <path d="M11 2c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.294 -2.333 5.588c0 3.704 3.134 6.706 7 6.706c3.866 0 7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" />
      </>;
    case 'mountain':
      return <>
        <path data-icon-part="mountain-outline" {...trace} d="M3 20h18l-6.921 -14.612a2.3 2.3 0 0 0 -4.158 0l-6.921 14.612" />
        <path data-icon-part="mountain-ridge" {...trace} d="M7.5 11l2 2.5l2.5 -2.5l2 3l2.5 -2" />
      </>;
    case 'rest':
      return <>
        <path data-icon-part="rest-pillow" style={joint(7, 11)} d="M5 9a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" />
        <path d="M22 17v-3h-20" />
        <path d="M2 8v9" />
        <path d="M12 14h10v-2a3 3 0 0 0 -3 -3h-7v5" />
      </>;
    case 'palette':
      return <>
        <path d="M12 21a9 9 0 0 1 0 -18c4.97 0 9 3.582 9 8c0 1.06 -.474 2.078 -1.318 2.828c-.844 .75 -1.989 1.172 -3.182 1.172h-2.5a2 2 0 0 0 -1 3.75a1.3 1.3 0 0 1 -1 2.25" />
        <g data-icon-part="palette-wells">
          <path d="M7.5 10.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" />
          <path d="M11.5 7.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" />
          <path d="M15.5 10.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0" />
        </g>
      </>;
    case 'language':
      return <>
        <g data-icon-part="language-first" style={joint(7.5, 8)}>
          <path d="M9 6.371c0 4.418 -2.239 6.629 -5 6.629" />
          <path d="M4 6.371h7" />
          <path d="M5 9c0 2.144 2.252 3.908 6 4" />
          <path d="M6.694 3l.793 .582" />
        </g>
        <g data-icon-part="language-second" style={joint(16, 15.5)}>
          <path d="M12 20l4 -9l4 9" />
          <path d="M19.1 18h-6.2" />
        </g>
      </>;
    case 'disconnect':
      return <>
        <path d="M17 22v-2" />
        <path d="M9 15l6 -6" />
        <g data-icon-part="disconnect-upper" style={joint(15, 9)}>
          <path d="M11 6l.463 -.536a5 5 0 0 1 7.071 7.072l-.534 .464" />
        </g>
        <g data-icon-part="disconnect-lower" style={joint(9, 15)}>
          <path d="M13 18l-.397 .534a5.068 5.068 0 0 1 -7.127 0a4.972 4.972 0 0 1 0 -7.071l.524 -.463" />
        </g>
        <path d="M20 17h2" />
        <path d="M2 7h2" />
        <path d="M7 2v2" />
      </>;
    case 'qr':
      return <>
        <path d="M4 5a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1l0 -4" />
        <path d="M7 17l0 .01" />
        <path d="M14 5a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1l0 -4" />
        <path d="M7 7l0 .01" />
        <path d="M4 15a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1l0 -4" />
        <path d="M17 7l0 .01" />
        <path d="M14 14l3 0" />
        <path d="M20 14l0 .01" />
        <path d="M14 14l0 3" />
        <path d="M14 20l3 0" />
        <path d="M17 17l3 0" />
        <path d="M20 17l0 3" />
        <path data-icon-part="qr-scan" style={joint(12, 4)} d="M4 4h16" strokeWidth={1} opacity={0} />
      </>;
    case 'contrast':
      return <>
        <path d="M3 12a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" />
        <path data-icon-part="contrast-half" style={joint(12, 12)} d="M12 17a5 5 0 0 0 0 -10v10" />
      </>;
    case 'finishFlag':
      return <g data-icon-part="finish-checker" style={joint(0, 12)}>
        <path className="flag-check" d="M0 0h6v6H0zM12 0h6v6h-6zM6 6h6v6H6zM18 6h6v6h-6zM0 12h6v6H0zM12 12h6v6h-6zM6 18h6v6H6zM18 18h6v6h-6z" />
      </g>;
    case 'strava':
      return <g data-icon-part="strava-mark" style={joint(12, 12)}>
        <path d="M15 13l-5 -10l-5 10m6 0l4 8l4 -8" />
      </g>;
    case 'brand':
      return <g data-icon-part="brand-mark" style={joint(32, 32)}>
        <path d="M32 4C8 4 4 8 4 32s4 28 28 28 28-4 28-28S56 4 32 4Z" />
        <rect x={12} y={12} width={40} height={40} rx={15} fill="var(--desk-on-action)" />
        <path d="M23.044 40.875 27.481 23.125H35.606Q37.581 23.125 38.879 23.775Q40.176 24.425 40.678 25.608Q41.18 26.79 40.777 28.4Q40.467 29.64 39.735 30.598Q39.004 31.555 37.956 32.165Q36.909 32.775 35.651 32.965L35.642 32.72Q37.467 32.72 38.2 33.5Q38.932 34.28 38.655 35.83L37.814 40.875H33.379L34.169 36.555Q34.359 35.555 33.96 35.08Q33.561 34.605 32.291 34.605H28.961L27.394 40.875ZM29.851 31.045H32.961Q34.266 31.045 35.11 30.47Q35.954 29.895 36.215 28.85Q36.482 27.78 35.914 27.232Q35.346 26.685 33.996 26.685H30.941Z" />
      </g>;
  }
}

export function CustomGlyph({ name, size }: { name: CustomIconName; size: number }): ReactElement {
  const filled = name === 'brand' || name === 'finishFlag';
  return <svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox={name === 'brand' ? '0 0 64 64' : '0 0 24 24'}
    fill={filled ? 'currentColor' : 'none'}
    stroke={filled ? 'none' : 'currentColor'}
    strokeWidth={2}
    strokeLinecap="round"
    strokeLinejoin="round"
    overflow={name === 'finishFlag' ? 'visible' : undefined}
    aria-hidden="true"
    focusable="false"
  >{artwork(name)}</svg>;
}

const rotation = (degrees: number): Keyframe[] => [
  { transform: 'rotate(0deg)', offset: 0 },
  { transform: `rotate(${degrees}deg)`, offset: 0.3 },
  { transform: `rotate(${-degrees}deg)`, offset: 0.65 },
  { transform: 'rotate(0deg)', offset: 1 },
];

const translation = (x: number, y: number): Keyframe[] => [
  { transform: 'translate(0px, 0px)', offset: 0 },
  { transform: `translate(${x}px, ${y}px)`, offset: 0.4 },
  { transform: 'translate(0px, 0px)', offset: 1 },
];

const strokeTrace = (): Keyframe[] => [
  { strokeDashoffset: '0', offset: 0 },
  { strokeDashoffset: '1', offset: 0.18 },
  { strokeDashoffset: '0', offset: 0.9 },
  { strokeDashoffset: '0', offset: 1 },
];

const stride = (phase: number) => 45 * (1 - Math.cos(phase * Math.PI * 2));
const knee = (phase: number) => (phase < 0.5 ? -20 : 65) * Math.sin(phase * Math.PI * 2) ** 2;
const gait = (transform: (phase: number) => string): Keyframe[] => Array.from({ length: 33 }, (_, frame) => ({ offset: frame / 32, transform: transform(frame / 32) }));
const angle = (degrees: number) => `rotate(${degrees.toFixed(3)}deg)`;

export const customMotions: Record<CustomIconName, Array<{ part: string; frames: Keyframe[] }>> = {
  run: [
    { part: 'run-body', frames: gait(phase => `translateY(${(0.2 * Math.sin(phase * Math.PI * 2) ** 2).toFixed(3)}px)`) },
    { part: 'run-leg-front', frames: gait(phase => angle(stride(phase))) },
    { part: 'run-leg-back', frames: gait(phase => angle(90 - stride(phase))) },
    { part: 'run-knee-front', frames: gait(phase => angle(knee(phase))) },
    { part: 'run-knee-back', frames: gait(phase => angle(knee((phase + 0.5) % 1))) },
    { part: 'run-arm-front', frames: gait(phase => angle(90 - stride(phase))) },
    { part: 'run-arm-back', frames: gait(phase => angle(stride(phase))) },
    { part: 'run-elbow-front', frames: gait(phase => angle(stride(phase) - 90)) },
    { part: 'run-elbow-back', frames: gait(phase => angle(-stride(phase))) },
  ],
  close: [{
    part: 'close-cross',
    frames: [{ transform: 'scale(1)', offset: 0 }, { transform: 'scale(.88)', offset: 0.35 }, { transform: 'scale(1)', offset: 1 }],
  }],
  bike: [
    { part: 'bike-wheel-back', frames: strokeTrace() },
    { part: 'bike-wheel-front', frames: strokeTrace() },
  ],
  strength: [{ part: 'strength-lift', frames: translation(0, -1) }],
  flame: [{ part: 'flame-inner', frames: rotation(3) }],
  mountain: [
    { part: 'mountain-outline', frames: strokeTrace() },
    { part: 'mountain-ridge', frames: strokeTrace() },
  ],
  rest: [{ part: 'rest-pillow', frames: translation(0, 0.5) }],
  palette: [{
    part: 'palette-wells',
    frames: [{ opacity: 1, offset: 0 }, { opacity: 0.75, offset: 0.25 }, { opacity: 1, offset: 1 }],
  }],
  language: [
    { part: 'language-first', frames: translation(-0.6, 0) },
    { part: 'language-second', frames: translation(0.6, 0) },
  ],
  disconnect: [
    { part: 'disconnect-upper', frames: translation(0.35, -0.35) },
    { part: 'disconnect-lower', frames: translation(-0.35, 0.35) },
  ],
  qr: [{
    part: 'qr-scan',
    frames: [
      { transform: 'translateY(0px)', opacity: 0, offset: 0 },
      { transform: 'translateY(0px)', opacity: 0.8, offset: 0.12 },
      { transform: 'translateY(16px)', opacity: 0.8, offset: 0.82 },
      { transform: 'translateY(16px)', opacity: 0, offset: 0.9 },
      { transform: 'translateY(0px)', opacity: 0, offset: 1 },
    ],
  }],
  contrast: [{ part: 'contrast-half', frames: rotation(6) }],
  finishFlag: [{
    part: 'finish-checker',
    frames: [
      { transform: 'skewY(0deg)', offset: 0 },
      { transform: 'skewY(3deg)', offset: 0.35 },
      { transform: 'skewY(0deg)', offset: 1 },
    ],
  }],
  strava: [{ part: 'strava-mark', frames: translation(0, -0.75) }],
  brand: [{ part: 'brand-mark', frames: translation(0, -2) }],
};
