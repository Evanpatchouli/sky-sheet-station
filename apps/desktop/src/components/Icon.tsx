const paths = {
  copy: "M9 9h11v12H9ZM4 15H3V3h11v2",
  paste: "M9 5H5v16h14V5h-4M9 3h6v4H9Z",
  delete: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7",
  save: "M4 3h13l4 4v14H3V3ZM7 3v6h10V3M7 21v-8h10v8",
  open: "M3 7h7l2 2h9l-3 11H3V7Zm0 0V4h7l2 3",
  audio:
    "M9 18V5l11-2v13M9 8l11-2M9 18c0 3-6 3-6 0s6-3 6 0Zm11-2c0 3-6 3-6 0s6-3 6 0Z",
  info: "M12 11v6m0-10v.1M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
  settings: "M4 7h16M4 17h16M8 4v6m8 4v6",
  play: "m8 4 12 8-12 8Z",
  pause: "M7 5v14M17 5v14",
  stop: "M6 6h12v12H6Z",
  loop: "M20 7H7a4 4 0 0 0-4 4m17-4-4-4m4 4-4 4M4 17h13a4 4 0 0 0 4-4M4 17l4 4m-4-4 4-4",
  close: "m6 6 12 12M6 18 18 6",
} as const;
export function Icon({ name }: { name: keyof typeof paths }) {
  return (
    <svg
      aria-hidden="true"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill={name === "play" || name === "stop" ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={paths[name]} />
    </svg>
  );
}
