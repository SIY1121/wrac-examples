/**
 * Echo Lama — Frontend
 *
 * Single XY pad that drives the Pitch (X) and Vowel (Y) parameters. The Gain,
 * Portamento, and Delay parameters live in the host parameter UI for now;
 * dedicated WebView controls will be added later in the trailing `1fr` strip.
 *
 * Communication uses the template2 command shape: subscribe via a Channel,
 * and identify parameters by stable numeric IDs (matching PARAM_*_ID on the
 * Rust side). Voicing has its own subscription so the character's mouth can
 * track the synth's active-voice state in real time.
 */
import { Channel, invoke } from "@novonotes/webview-bridge";
import "./style.css";

// --- Message shapes pushed from Rust. Keep in sync with notifier.rs. ---

type ParameterState = {
  type: "parameter-value";
  parameterId: number;
  value: number;
  text: string;
};

type VoicingState = {
  type: "voicing-state";
  value: boolean;
};

type SubscribeResponse = {
  ok?: boolean;
  subscriptionId: number;
};

// Parameter IDs. Keep in sync with src-plugin/src/plugin/parameters.rs.
const PARAM_PITCH_ID = 2;
const PARAM_VOWEL_ID = 3;

// Parameter ranges. Mirror MIN_PITCH / MAX_PITCH and MIN_VOWEL / MAX_VOWEL on
// the Rust side; the pad uses these to map pointer coordinates back to values.
const PITCH_MIN = -12;
const PITCH_MAX = 12;
const VOWEL_MIN = 0;
const VOWEL_MAX = 1;

// --- DOM ---

function queryRequired<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) {
    throw new Error(`required element not found: ${selector}`);
  }
  return el;
}

const pad = queryRequired<HTMLDivElement>("#xy-pad");
const puck = queryRequired<HTMLDivElement>("#pad-puck");
const pitchLabel = queryRequired<HTMLSpanElement>("#pitch-text");
const vowelLabel = queryRequired<HTMLSpanElement>("#vowel-text");
const characterMouth = queryRequired<HTMLDivElement>("#character-mouth");
const browPathLeft = queryRequired<SVGPathElement>("#brow-path-left");
const browPathRight = queryRequired<SVGPathElement>("#brow-path-right");

// --- State ---

let pitch = 0;
let vowel = 0.5;
let pitchText = "+0.00 st";
let vowelText = "A";
let voicing = false;

let dragging = false;
let pitchGestureActive = false;
let vowelGestureActive = false;

let parameterSubscriptionId: number | undefined;
let voicingSubscriptionId: number | undefined;

// -----------------------------------------------------------------------
// Subscribe to Rust → JS push notifications
// -----------------------------------------------------------------------

const parameterChannel = new Channel<ParameterState>((message) => {
  if (!message || message.type !== "parameter-value") {
    return;
  }
  switch (message.parameterId) {
    case PARAM_PITCH_ID:
      pitch = clamp(message.value, PITCH_MIN, PITCH_MAX);
      pitchText = message.text;
      renderPitch();
      break;
    case PARAM_VOWEL_ID:
      vowel = clamp(message.value, VOWEL_MIN, VOWEL_MAX);
      vowelText = message.text;
      renderVowel();
      break;
    default:
      // Other parameters (Gain, Portamento, Delay*) currently have no WebView
      // representation; the host UI exposes them.
      break;
  }
});

const voicingChannel = new Channel<VoicingState>((message) => {
  if (message && message.type === "voicing-state") {
    voicing = message.value;
    updateCharacter();
  }
});

void (async () => {
  // Pull current values before subscribing so the initial render isn't stuck
  // on the design-time placeholder.
  try {
    const pitchState = await invoke<ParameterState>("get_parameter_state", {
      parameterId: PARAM_PITCH_ID,
    });
    pitch = clamp(pitchState.value, PITCH_MIN, PITCH_MAX);
    pitchText = pitchState.text;
    renderPitch();

    const vowelState = await invoke<ParameterState>("get_parameter_state", {
      parameterId: PARAM_VOWEL_ID,
    });
    vowel = clamp(vowelState.value, VOWEL_MIN, VOWEL_MAX);
    vowelText = vowelState.text;
    renderVowel();
  } catch {
    // Initial fetch failures are non-fatal; the periodic timer will catch up.
  }

  const paramSub = await invoke<SubscribeResponse>("subscribe_parameters", {
    channel: parameterChannel,
  });
  parameterSubscriptionId = paramSub.subscriptionId;

  const voicingSub = await invoke<SubscribeResponse>("subscribe_voicing", {
    channel: voicingChannel,
  });
  voicingSubscriptionId = voicingSub.subscriptionId;

  await invoke("write_to_log", { message: "Echo Lama GUI initialized" });
})();

window.addEventListener("beforeunload", () => {
  if (pitchGestureActive) endGesture(PARAM_PITCH_ID);
  if (vowelGestureActive) endGesture(PARAM_VOWEL_ID);
  if (parameterSubscriptionId !== undefined) {
    void invoke("unsubscribe_gui_subscription", {
      subscriptionId: parameterSubscriptionId,
    });
  }
  if (voicingSubscriptionId !== undefined) {
    void invoke("unsubscribe_gui_subscription", {
      subscriptionId: voicingSubscriptionId,
    });
  }
});

// -----------------------------------------------------------------------
// Rendering
// -----------------------------------------------------------------------

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function renderPitch(): void {
  pitchLabel.textContent = pitchText;
  updatePuck();
}

function renderVowel(): void {
  vowelLabel.textContent = vowelText;
  updatePuck();
  updateCharacter();
}

function updatePuck(): void {
  const xRatio = (pitch - PITCH_MIN) / (PITCH_MAX - PITCH_MIN);
  const yRatio = 1 - (vowel - VOWEL_MIN) / (VOWEL_MAX - VOWEL_MIN);
  puck.style.left = `${xRatio * 100}%`;
  puck.style.top = `${yRatio * 100}%`;
}

type MouthShape = { readonly x: number; readonly y: number };
const VOWEL_MOUTH_SHAPES: readonly MouthShape[] = [
  { x: 0.45, y: 0.65 }, // U
  { x: 0.7, y: 0.85 }, // O
  { x: 1.0, y: 1.25 }, // A
  { x: 1.15, y: 0.55 }, // E
  { x: 1.3, y: 0.25 }, // I
];

const CLOSED_MOUTH_SCALE = { x: 0.85, y: 0.12 } as const;

type BrowShape = {
  readonly inner: number;
  readonly middle: number;
  readonly outer: number;
};
type VowelBrowFrame = { readonly left: BrowShape; readonly right: BrowShape };
const VOWEL_BROW_SHAPES: readonly VowelBrowFrame[] = [
  // U
  {
    left: { inner: -3, middle: -2, outer: -3 },
    right: { inner: 3, middle: 4, outer: 4 },
  },
  // O
  {
    left: { inner: -1, middle: 1, outer: -1 },
    right: { inner: -3, middle: -2, outer: 1 },
  },
  // A
  {
    left: { inner: 3, middle: 6, outer: 3 },
    right: { inner: 3, middle: 6, outer: 3 },
  },
  // E
  {
    left: { inner: 3, middle: 3, outer: 1 },
    right: { inner: 3, middle: 3, outer: 1 },
  },
  // I
  {
    left: { inner: 7, middle: 0, outer: -2 },
    right: { inner: 7, middle: 0, outer: -2 },
  },
];

const BROW_X = { inner: 36, middle: 20, outer: 4 } as const;
const BROW_BASE_Y = 15;

function mouthShapeForVowel(value: number): MouthShape {
  const n = VOWEL_MOUTH_SHAPES.length;
  const norm = (value - VOWEL_MIN) / (VOWEL_MAX - VOWEL_MIN);
  const position = clamp(norm, 0, 1) * (n - 1);
  const i = Math.min(n - 2, Math.floor(position));
  const t = position - i;
  const a = VOWEL_MOUTH_SHAPES[i];
  const b = VOWEL_MOUTH_SHAPES[i + 1];
  return {
    x: a.x * (1 - t) + b.x * t,
    y: a.y * (1 - t) + b.y * t,
  };
}

function updateCharacter(): void {
  updateBrows();

  if (voicing || dragging) {
    characterMouth.classList.remove("closed");
    const shape = mouthShapeForVowel(vowel);
    characterMouth.style.transform = `scale(${shape.x.toFixed(3)}, ${shape.y.toFixed(3)})`;
  } else {
    characterMouth.classList.add("closed");
    characterMouth.style.transform = `scale(${CLOSED_MOUTH_SCALE.x}, ${CLOSED_MOUTH_SCALE.y})`;
  }
}

function lerp(a: number, b: number, t: number): number {
  return a * (1 - t) + b * t;
}

function lerpBrowShape(a: BrowShape, b: BrowShape, t: number): BrowShape {
  return {
    inner: lerp(a.inner, b.inner, t),
    middle: lerp(a.middle, b.middle, t),
    outer: lerp(a.outer, b.outer, t),
  };
}

function browFramesForVowel(value: number): VowelBrowFrame {
  const n = VOWEL_BROW_SHAPES.length;
  const norm = clamp((value - VOWEL_MIN) / (VOWEL_MAX - VOWEL_MIN), 0, 1);
  const position = norm * (n - 1);
  const i = Math.min(n - 2, Math.floor(position));
  const t = position - i;
  const a = VOWEL_BROW_SHAPES[i];
  const b = VOWEL_BROW_SHAPES[i + 1];
  return {
    left: lerpBrowShape(a.left, b.left, t),
    right: lerpBrowShape(a.right, b.right, t),
  };
}

function buildBrowPath(shape: BrowShape): string {
  const innerY = BROW_BASE_Y - shape.inner;
  const middleY = BROW_BASE_Y - shape.middle;
  const outerY = BROW_BASE_Y - shape.outer;
  const controlX = 2 * BROW_X.middle - 0.5 * (BROW_X.inner + BROW_X.outer);
  const controlY = 2 * middleY - 0.5 * (innerY + outerY);
  return `M ${BROW_X.inner} ${innerY.toFixed(2)} Q ${controlX} ${controlY.toFixed(2)} ${BROW_X.outer} ${outerY.toFixed(2)}`;
}

function updateBrows(): void {
  const frame = browFramesForVowel(vowel);
  browPathLeft.setAttribute("d", buildBrowPath(frame.left));
  browPathRight.setAttribute("d", buildBrowPath(frame.right));
}

// -----------------------------------------------------------------------
// Gestures
// -----------------------------------------------------------------------

function beginGesture(parameterId: number): void {
  if (parameterId === PARAM_PITCH_ID) {
    if (pitchGestureActive) return;
    pitchGestureActive = true;
  } else if (parameterId === PARAM_VOWEL_ID) {
    if (vowelGestureActive) return;
    vowelGestureActive = true;
  }
  void invoke("begin_parameter_gesture", { parameterId });
}

function endGesture(parameterId: number): void {
  if (parameterId === PARAM_PITCH_ID) {
    if (!pitchGestureActive) return;
    pitchGestureActive = false;
  } else if (parameterId === PARAM_VOWEL_ID) {
    if (!vowelGestureActive) return;
    vowelGestureActive = false;
  }
  void invoke("end_parameter_gesture", { parameterId });
}

function applyPitch(value: number): void {
  pitch = clamp(value, PITCH_MIN, PITCH_MAX);
  // Local pre-render for responsiveness; Rust will broadcast the formatted
  // text back, which overwrites this rough preview.
  pitchText = `${pitch >= 0 ? "+" : ""}${pitch.toFixed(2)} st`;
  renderPitch();
  void invoke("set_parameter_value", {
    parameterId: PARAM_PITCH_ID,
    value: pitch,
  });
}

function applyVowel(value: number): void {
  vowel = clamp(value, VOWEL_MIN, VOWEL_MAX);
  vowelText = closestVowelLabel(vowel);
  renderVowel();
  void invoke("set_parameter_value", {
    parameterId: PARAM_VOWEL_ID,
    value: vowel,
  });
}

const VOWEL_LABELS = ["U", "O", "A", "E", "I"] as const;
function closestVowelLabel(v: number): string {
  const norm = (v - VOWEL_MIN) / (VOWEL_MAX - VOWEL_MIN);
  const idx = Math.min(
    VOWEL_LABELS.length - 1,
    Math.max(0, Math.round(norm * (VOWEL_LABELS.length - 1))),
  );
  return VOWEL_LABELS[idx];
}

// -----------------------------------------------------------------------
// Pointer interaction
// -----------------------------------------------------------------------

function updateFromPointer(event: PointerEvent): void {
  const rect = pad.getBoundingClientRect();
  const xRatio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
  const yRatio = clamp((event.clientY - rect.top) / rect.height, 0, 1);
  const nextPitch = PITCH_MIN + xRatio * (PITCH_MAX - PITCH_MIN);
  const nextVowel = VOWEL_MIN + (1 - yRatio) * (VOWEL_MAX - VOWEL_MIN);
  applyPitch(nextPitch);
  applyVowel(nextVowel);
}

pad.addEventListener("pointerdown", (event) => {
  dragging = true;
  pad.setPointerCapture(event.pointerId);
  beginGesture(PARAM_PITCH_ID);
  beginGesture(PARAM_VOWEL_ID);
  updateFromPointer(event);
});

pad.addEventListener("pointermove", (event) => {
  if (!dragging) return;
  updateFromPointer(event);
});

const finishDrag = (event: PointerEvent) => {
  if (!dragging) return;
  dragging = false;
  pad.releasePointerCapture(event.pointerId);
  endGesture(PARAM_PITCH_ID);
  endGesture(PARAM_VOWEL_ID);
  updateCharacter();
};

pad.addEventListener("pointerup", finishDrag);
pad.addEventListener("pointercancel", finishDrag);

// Initial render (overwritten as soon as the subscribe pushes real state).
renderPitch();
renderVowel();
