import { memo, useId } from "react";
import type { CSSProperties } from "react";
import "../styles/music-progress.css";

// Decorative notation only: it does not represent inferred notes or task progress.
const notes = [
  { x: 35, y: 58, flag: true },
  { x: 100, y: 42 },
  { x: 154, y: 34 },
  { x: 207, y: 52, flag: true },
  { x: 287, y: 65 },
  { x: 341, y: 49 },
  { x: 403, y: 38, flag: true },
  { x: 455, y: 46 },
];

const RollingStaff = memo(function RollingStaff() {
  const pattern = useId();
  return (
    <div className="music-progress-window" aria-hidden="true">
      <svg className="music-progress-score" height="96" focusable="false">
        <defs>
          <pattern
            id={pattern}
            width="480"
            height="96"
            patternUnits="userSpaceOnUse"
          >
            <g
              className="music-progress-lines"
              fill="none"
              stroke="currentColor"
              strokeWidth="0.8"
            >
              {[30, 39, 48, 57, 66].map((y) => (
                <path
                  key={y}
                  d={`M0 ${y} C80 ${y - 7} 160 ${y - 7} 240 ${y} S400 ${y + 7} 480 ${y}`}
                />
              ))}
              <path d="M248 30.7v36M474 30.5v36" />
            </g>
            <g
              className="music-progress-notes"
              fill="currentColor"
              stroke="currentColor"
              strokeWidth="1.35"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {notes.map((note) => (
                <g key={note.x}>
                  <ellipse
                    cx={note.x}
                    cy={note.y}
                    rx="5.2"
                    ry="3.5"
                    stroke="none"
                    transform={`rotate(-24 ${note.x} ${note.y})`}
                  />
                  <path d={`M${note.x + 4.6} ${note.y - 1}v-27`} fill="none" />
                  {note.flag && (
                    <path
                      d={`M${note.x + 4.6} ${note.y - 28}c0 9 13 9 6 20`}
                      fill="none"
                    />
                  )}
                </g>
              ))}
              <path d="M104.6 14 L158.6 6 L158.6 9 L104.6 17Z" stroke="none" />
              <path
                d="M291.6 37 L345.6 21 L345.6 24 L291.6 40Z"
                stroke="none"
              />
            </g>
          </pattern>
        </defs>
        <rect width="100%" height="96" fill={`url(#${pattern})`} />
      </svg>
    </div>
  );
});

/** `value` is the measured percentage of the current stage; omit when unknown. */
export function MusicProgress({
  label,
  value,
}: {
  label: string;
  value?: number;
}) {
  const labelId = useId();
  const percent =
    value !== undefined && Number.isFinite(value)
      ? Math.max(0, Math.min(100, value))
      : undefined;
  return (
    <div className="music-progress" data-complete={percent === 100}>
      <div className="music-progress-heading">
        <span id={labelId} role="status">
          {label}
        </span>
        <span className="music-progress-value" aria-hidden="true">
          {percent === undefined ? (
            "处理中"
          ) : (
            <>
              当前阶段{" "}
              <b>
                {Math.round(percent)}
                <small>%</small>
              </b>
            </>
          )}
        </span>
      </div>
      <div
        role="progressbar"
        aria-labelledby={labelId}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        aria-valuetext={
          percent === undefined
            ? "处理中，进度未知"
            : `当前阶段 ${Math.round(percent)}%`
        }
      >
        <RollingStaff />
        <div className="music-progress-rail" aria-hidden="true">
          <div
            className="music-progress-fill"
            data-indeterminate={percent === undefined}
            style={{ "--progress": (percent ?? 0) / 100 } as CSSProperties}
          />
        </div>
      </div>
    </div>
  );
}
