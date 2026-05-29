import type { MembershipFilter } from "@/lib/ipc/types";

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
    <div role="radiogroup" className="flex items-center gap-1 text-xs">
      {OPTIONS.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className="pill px-3 py-1"
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
