import { useEffect, useState } from "react";
import type { ReactNode } from "react";

type StatusKind = "mic" | "spinner" | "success" | "error";

type WhisperingStatus = {
  kind: StatusKind;
  message: string;
  level?: number;
};

type NonMicStatusKind = Exclude<StatusKind, "mic" | "spinner">;

declare global {
  interface Window {
    setWhisperingStatus?: (status: WhisperingStatus) => void;
  }
}

const icons: Record<NonMicStatusKind, ReactNode> = {
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

const METER_SEGMENTS = 12;

function meterSegmentFill(level: number, index: number) {
  const normalized = Math.max(0, Math.min(1, level));
  return Math.max(0, Math.min(1, normalized * METER_SEGMENTS - index));
}

export function App() {
  const [status, setStatus] = useState<WhisperingStatus>({
    kind: "spinner",
    message: "Loading",
    level: 0,
  });

  useEffect(() => {
    window.setWhisperingStatus = setStatus;
    return () => {
      delete window.setWhisperingStatus;
    };
  }, []);

  return (
    <main className="overlay-shell">
      <section className={`pill ${status.kind === "mic" ? "pill-mic" : ""}`} aria-live="polite">
        {status.kind === "mic" ? (
          <>
            <div className="mic-badge" aria-hidden="true">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.1"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 4a3 3 0 0 0-3 3v4.5a3 3 0 0 0 6 0V7a3 3 0 0 0-3-3Z" />
                <path d="M18 10.5a6 6 0 0 1-12 0" />
                <path d="M12 16.5V20" />
              </svg>
            </div>
            <div className="recording-content">
              <div className="message">{status.message}</div>
              <div className="level-track" aria-hidden="true">
                {Array.from({ length: METER_SEGMENTS }, (_, index) => {
                  const fill = meterSegmentFill(status.level ?? 0, index);
                  return (
                    <span
                      key={index}
                      className={`level-segment ${fill > 0 ? "active" : ""}`}
                      style={{
                        opacity: 0.22 + fill * 0.78,
                        transform: `scaleY(${0.84 + fill * 0.16})`,
                      }}
                    />
                  );
                })}
              </div>
            </div>
          </>
        ) : (
          <>
            <div className={`icon ${status.kind === "error" ? "error" : ""}`}>
              {status.kind === "spinner" ? (
                <div className="spinner" />
              ) : status.kind === "success" ? (
                icons.success
              ) : (
                icons.error
              )}
            </div>
            <div className="message">{status.message}</div>
          </>
        )}
      </section>
    </main>
  );
}
