import { Component, type ErrorInfo, type ReactNode } from "react";

/**
 * Catches render-time crashes so a single bad component cannot leave the user
 * staring at a blank window.
 *
 * Deliberately dependency-free: it must not rely on any context that might be
 * the thing that crashed, so the copy is picked from `<html lang>` directly.
 */
export default class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[vidaro] render error:", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <CrashScreen
        error={this.state.error}
        onRetry={() => this.setState({ error: null })}
      />
    );
  }
}

function CrashScreen({ error, onRetry }: { error: Error; onRetry: () => void }) {
  const fa = document.documentElement.lang === "fa";
  const text = fa
    ? {
        title: "خطایی رخ داد",
        hint: "رابط کاربری به خطای غیرمنتظره برخورد کرد. دانلودهای در جریان در پس‌زمینه ادامه می‌یابند.",
        reload: "بارگذاری مجدد",
        retry: "تلاش مجدد",
        copy: "کپی جزئیات",
        copied: "کپی شد",
      }
    : {
        title: "Something went wrong",
        hint: "The interface hit an unexpected error. Downloads already running continue in the background.",
        reload: "Reload",
        retry: "Try again",
        copy: "Copy details",
        copied: "Copied",
      };

  const details = `${error.name}: ${error.message}\n${error.stack ?? ""}`;

  async function copy() {
    try {
      await navigator.clipboard.writeText(details);
      const target = document.getElementById("crash-copy");
      if (target) {
        target.textContent = text.copied;
        window.setTimeout(() => {
          target.textContent = text.copy;
        }, 1600);
      }
    } catch {
      /* clipboard unavailable — the message is on screen anyway */
    }
  }

  return (
    <div className="crash">
      <div className="crash-card">
        <div className="crash-icon">!</div>
        <h1>{text.title}</h1>
        <p className="muted">{text.hint}</p>

        <pre className="crash-details" dir="ltr">
          {details}
        </pre>

        <div className="crash-actions">
          <button className="btn primary" onClick={() => window.location.reload()}>
            {text.reload}
          </button>
          <button className="btn secondary" onClick={onRetry}>
            {text.retry}
          </button>
          <button className="ghost-btn" id="crash-copy" onClick={() => void copy()}>
            {text.copy}
          </button>
        </div>
      </div>
    </div>
  );
}
