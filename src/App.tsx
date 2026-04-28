import { useEffect, useState } from "react";
import type { ReactNode } from "react";

type StatusKind = "mic" | "spinner" | "success" | "error";

type WhisperingStatus = {
  kind: StatusKind;
  message: string;
};

declare global {
  interface Window {
    setWhisperingStatus?: (status: WhisperingStatus) => void;
  }
}

const icons: Record<Exclude<StatusKind, "spinner">, ReactNode> = {
  mic: (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3Z" />
      <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
      <path d="M12 19v3" />
    </svg>
  ),
  success: (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  ),
  error: <span>!</span>,
};

export function App() {
  const [status, setStatus] = useState<WhisperingStatus>({
    kind: "spinner",
    message: "Loading",
  });

  useEffect(() => {
    window.setWhisperingStatus = setStatus;
    return () => {
      delete window.setWhisperingStatus;
    };
  }, []);

  return (
    <main className="overlay-shell">
      <section className="pill" aria-live="polite">
        <div className={`icon ${status.kind === "error" ? "error" : ""}`}>
          {status.kind === "spinner" ? <div className="spinner" /> : icons[status.kind]}
        </div>
        <div className="message">{status.message}</div>
      </section>
    </main>
  );
}
