import { Component, type ErrorInfo, type ReactNode } from 'react';

type Props = { children: ReactNode };
type State = { error: Error | null };

/**
 * Catches render errors so a single bad component cannot blank the whole panel.
 *
 * Skrab is a tray app with no visible console — a white screen is indistinguishable
 * from a hang, and the only recovery would be quitting from the tray. Showing the
 * message and a reload button keeps the app usable and makes bug reports specific.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Skrab render error:', error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <h1 className="text-sm font-semibold">Something broke in the interface</h1>
        <p className="text-muted-foreground max-w-xs text-xs" data-selectable>
          {error.message}
        </p>
        <p className="text-muted-foreground text-[11px]">
          Your clipboard history is safe — this is only the display.
        </p>
        <button
          type="button"
          onClick={() => {
            this.setState({ error: null });
            window.location.reload();
          }}
          className="gradient-brand mt-1 rounded-md px-3 py-1.5 text-xs font-medium text-white"
        >
          Reload
        </button>
      </div>
    );
  }
}
