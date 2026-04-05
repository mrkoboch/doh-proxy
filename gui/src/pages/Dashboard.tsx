import type { ProxyStatus } from "../lib/types";

interface Props {
  status: ProxyStatus;
  onStatusChange: () => void;
}

export function Dashboard(_props: Props) {
  return <div className="p-6 text-secondary text-sm">Dashboard</div>;
}
