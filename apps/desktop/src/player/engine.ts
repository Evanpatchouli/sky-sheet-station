import { activeKeys, lowerBound, tickAt } from "./model.ts";
import type { PlayerNote, PreparedScore } from "./model.ts";

export interface PlaybackState {
  status: "stopped" | "playing" | "paused";
  position: number;
  keys: number[];
  speed: number;
  loop: boolean;
  error: string | null;
}
export class PlayerEngine {
  private context: AudioContext | null = null;
  private master: GainNode | null = null;
  private voices = new Set<OscillatorNode>();
  private score: PreparedScore | null = null;
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private anchorTime = 0;
  private anchorPosition = 0;
  private next = 0;
  private generation = 0;
  private volume = 0.65;
  private state: PlaybackState = {
    status: "stopped",
    position: 0,
    keys: [],
    speed: 1,
    loop: false,
    error: null,
  };
  subscribe = (fn: () => void) => {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  };
  snapshot = () => this.state;
  private publish(update: Partial<PlaybackState>) {
    this.state = { ...this.state, ...update };
    for (const fn of this.listeners) fn();
  }
  private position() {
    return this.state.status === "playing" && this.context
      ? this.anchorPosition +
          (this.context.currentTime - this.anchorTime) * this.state.speed
      : this.state.position;
  }
  private silence() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    for (const voice of this.voices) {
      try {
        voice.stop();
      } catch {
        /* already ended */
      }
    }
    this.voices.clear();
  }
  load(score: PreparedScore) {
    this.stop();
    this.score = score;
    this.publish({ error: score.playbackError });
  }
  async play() {
    if (
      !this.score ||
      this.score.playbackError ||
      this.score.duration === 0 ||
      this.state.status === "playing"
    )
      return;
    const generation = ++this.generation;
    try {
      if (!this.context) {
        this.context = new AudioContext();
        this.master = this.context.createGain();
        const limiter = this.context.createDynamicsCompressor();
        this.master.connect(limiter);
        limiter.connect(this.context.destination);
        this.master.gain.value = this.volume;
        this.context.onstatechange = () => {
          if (
            this.context?.state === "suspended" &&
            this.state.status === "playing"
          )
            this.pause();
        };
      }
      await this.context.resume();
      if (generation !== this.generation) return;
      const position =
        this.state.position >= this.score.duration ? 0 : this.state.position;
      this.restart(position);
    } catch (error) {
      this.publish({
        status: "paused",
        error: `无法启动本地音频：${String(error)}`,
      });
    }
  }
  private restart(position: number) {
    this.silence();
    if (!this.context || !this.score) return;
    this.anchorPosition = position;
    this.anchorTime = this.context.currentTime;
    this.next = lowerBound(this.score.notes, position);
    this.publish({ status: "playing", position, error: null });
    const tick = tickAt(position, this.score.score);
    for (const note of this.score.index.between(tick, tick)) {
      if (note.start < position) this.schedule(note, position);
    }
    this.pump();
    if (this.state.status === "playing")
      this.timer = setInterval(() => this.pump(), 25);
  }
  private schedule(note: PlayerNote, position: number) {
    if (!this.context || !this.master || !this.score) return;
    const now = this.context.currentTime;
    const when = Math.max(
      now,
      this.anchorTime + (note.start - this.anchorPosition) / this.state.speed,
    );
    const end =
      this.anchorTime + (note.end - this.anchorPosition) / this.state.speed;
    if (end <= now || note.end <= position) return;
    const oscillator = this.context.createOscillator(),
      gain = this.context.createGain();
    oscillator.type = "triangle";
    oscillator.frequency.value = 440 * 2 ** ((note.midi - 69) / 12);
    const length = end - when;
    const attack = Math.min(0.004, length / 4);
    gain.gain.setValueAtTime(0, when);
    // Fixed local audition level; no velocity is added to the Score model.
    gain.gain.linearRampToValueAtTime(0.16, when + attack);
    if (this.score.score.instrument.playback_mode === "trigger") {
      gain.gain.exponentialRampToValueAtTime(
        0.001,
        when + Math.max(attack, Math.min(length * 0.9, 1.2 / this.state.speed)),
      );
    } else {
      gain.gain.setValueAtTime(
        0.16,
        Math.max(when + attack, end - Math.min(0.02, length / 4)),
      );
    }
    gain.gain.linearRampToValueAtTime(0, end);
    oscillator.connect(gain);
    gain.connect(this.master);
    this.voices.add(oscillator);
    oscillator.onended = () => {
      this.voices.delete(oscillator);
      oscillator.disconnect();
      gain.disconnect();
    };
    oscillator.start(when);
    oscillator.stop(end);
  }
  private pump() {
    if (!this.score || this.state.status !== "playing") return;
    const position = this.position();
    if (position >= this.score.duration) {
      if (this.state.loop) {
        this.restart(position % this.score.duration);
      } else {
        this.silence();
        this.publish({
          status: "stopped",
          position: this.score.duration,
          keys: [],
        });
      }
      return;
    }
    const horizon = position + 0.12 * this.state.speed;
    while (
      this.next < this.score.notes.length &&
      this.score.notes[this.next].start < horizon
    ) {
      this.schedule(this.score.notes[this.next++], position);
    }
    this.publish({ position, keys: activeKeys(this.score, position) });
  }
  pause() {
    ++this.generation;
    if (this.state.status !== "playing") return;
    const position = this.score
      ? Math.min(this.position(), this.score.duration)
      : 0;
    this.silence();
    this.publish({
      status: "paused",
      position,
      keys:
        this.score && !this.score.playbackError && position < this.score.duration
          ? activeKeys(this.score, position)
          : [],
    });
  }
  stop() {
    ++this.generation;
    this.silence();
    this.publish({ status: "stopped", position: 0, keys: [] });
  }
  seek(position: number) {
    if (!this.score || this.score.playbackError) return;
    const next = Math.max(0, Math.min(position, this.score.duration));
    if (this.state.status === "playing") this.restart(next);
    else
      this.publish({
        position: next,
        keys: next < this.score.duration ? activeKeys(this.score, next) : [],
      });
  }
  setSpeed(speed: number) {
    if (![0.5, 0.75, 1, 1.25, 1.5, 2].includes(speed)) return;
    const position = this.position();
    this.publish({ speed });
    if (this.state.status === "playing") this.restart(position);
  }
  setLoop(loop: boolean) {
    this.publish({ loop });
  }
  setVolume(volume: number) {
    this.volume = Math.max(0, Math.min(1, volume));
    if (this.master && this.context)
      this.master.gain.setTargetAtTime(
        this.volume,
        this.context.currentTime,
        0.01,
      );
  }
  dispose() {
    this.stop();
    void this.context?.close();
    this.context = null;
    this.master = null;
    this.listeners.clear();
  }
}
