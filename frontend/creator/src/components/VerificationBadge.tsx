import type {VerificationStatus} from "../types";

const BADGES: Record<VerificationStatus, {text: string; className: string}> = {
    verified: {text: "Verified", className: "badge-success"},
    pending: {text: "Awaiting check", className: "badge-warning"},
    failed: {text: "Tag not found", className: "badge-error"},
};

export function VerificationBadge(props: {status: VerificationStatus}) {
    const badge = BADGES[props.status];
    return <span className={`badge ${badge.className}`}>{badge.text}</span>;
}
