import { Music } from "lucide-react";
import { cn } from "@/lib/cn";

type Props = {
  onClick: () => void;
  isLoading: boolean;
  error?: string;
};

export function LoginScreen({ onClick, isLoading, error }: Props) {
  return (
    <main className="flex h-full flex-1 items-center justify-center">
      <div className="flex max-w-sm flex-col items-center gap-6 text-center">
        <div className="rounded-full bg-emerald-600/15 p-4 text-emerald-400">
          <Music size={32} />
        </div>
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold tracking-tight">Spotify Archivist</h1>
          <p className="text-sm text-neutral-400">
            Watches your Liked Songs and playlists. Flags anything Spotify silently removes.
          </p>
        </div>
        <button
          type="button"
          disabled={isLoading}
          onClick={onClick}
          className={cn(
            "rounded-full bg-emerald-500 px-6 py-2 text-sm font-medium text-neutral-950",
            "transition hover:bg-emerald-400 disabled:opacity-50",
          )}
        >
          {isLoading ? "Waiting for browser…" : "Login with Spotify"}
        </button>
        {error ? <p className="text-sm text-rose-400">{error}</p> : null}
      </div>
    </main>
  );
}
