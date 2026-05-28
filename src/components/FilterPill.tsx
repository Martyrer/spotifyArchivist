import type { MembershipFilter } from "@/lib/ipc/types";
import { cn } from "@/lib/cn";

const OPTIONS: { value: MembershipFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "present", label: "Present" },
  { value: "removed", label: "Removed" },
];

type Props = {
  value: MembershipFilter;
  onChange: (v: MembershipFilter) => void;
};

export function FilterPill({ value, onChange }: Props) {
  return (
    <div role="radiogroup" className="flex rounded-full border border-neutral-700 p-0.5 text-xs">
      {OPTIONS.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className={cn(
            "rounded-full px-3 py-1 transition",
            value === o.value
              ? "bg-emerald-500 text-neutral-950"
              : "text-neutral-400 hover:text-neutral-100",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
