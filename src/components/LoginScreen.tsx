import { Music } from "lucide-react";

type Props = {
  onClick: () => void;
  onCancel?: () => void;
  isLoading: boolean;
  error?: string;
};

export function LoginScreen({ onClick, onCancel, isLoading, error }: Props) {
  return (
    <main className="flex h-full flex-1 items-center justify-center bg-bg text-fg">
      <div className="fc flex max-w-sm flex-col items-center gap-6 border border-border bg-surface px-8 py-10 text-center">
        <div className="grid size-14 place-items-center bg-surface-2 text-accent">
          <Music size={28} />
        </div>
        <div className="space-y-1">
          <h1 className="text-xl font-semibold tracking-tight">Spotify Archivist</h1>
          <p className="text-sm text-muted">
            Watches your Liked Songs and playlists. Flags anything Spotify silently removes.
          </p>
        </div>
        <button
          type="button"
          disabled={isLoading}
          onClick={onClick}
          data-active={isLoading ? "true" : undefined}
          className="pill px-6 py-2 text-sm font-medium"
        >
          {isLoading ? "Waiting for browser…" : "Login with Spotify"}
        </button>
        {isLoading && onCancel ? (
          <button
            type="button"
            onClick={onCancel}
            className="text-xs text-muted underline-offset-4 transition-colors duration-200 ease-out hover:text-fg hover:underline"
          >
            Cancel
          </button>
        ) : null}
        {error ? <p className="font-mono text-xs text-muted">{error}</p> : null}
      </div>
    </main>
  );
}
