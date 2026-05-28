import { appName } from "@/lib/branding";

export default function App() {
  return (
    <main className="flex h-full w-full items-center justify-center">
      <div className="flex flex-col items-center gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">{appName}</h1>
        <p className="text-sm text-neutral-400">Pre-implementation shell.</p>
      </div>
    </main>
  );
}
