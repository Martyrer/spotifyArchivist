import { useMemo } from "react";
import { ART_SIZE, generateTrackArt, trackArtSeed } from "@/lib/trackArt";

type Props = {
  name: string;
  artists: string;
  album: string;
  className?: string;
};

export function TrackArt({ name, artists, album, className }: Props) {
  const spec = useMemo(
    () => generateTrackArt(trackArtSeed(name, artists, album)),
    [name, artists, album],
  );
  return (
    <svg
      viewBox={`0 0 ${ART_SIZE} ${ART_SIZE}`}
      className={className}
      role="img"
      aria-hidden="true"
      preserveAspectRatio="xMidYMid slice"
    >
      <rect width={ART_SIZE} height={ART_SIZE} fill={spec.bg} />
      {spec.shapes.map((s, i) => {
        if (s.kind === "circle") {
          return <circle key={i} cx={s.cx} cy={s.cy} r={s.r} fill={s.color} opacity={0.9} />;
        }
        if (s.kind === "rect") {
          return <rect key={i} x={s.x} y={s.y} width={s.w} height={s.h} fill={s.color} opacity={0.9} />;
        }
        return <polygon key={i} points={s.points} fill={s.color} opacity={0.9} />;
      })}
    </svg>
  );
}
