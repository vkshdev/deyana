import markUrl from "../../../../../assets/brand/deyana-mark-ui-256.png";

interface DeyanaMarkProps {
  className?: string;
}

export function DeyanaMark({ className }: DeyanaMarkProps) {
  const classes = ["deyana-mark", className].filter(Boolean).join(" ");

  return <img className={classes} src={markUrl} alt="" aria-hidden="true" draggable={false} />;
}
