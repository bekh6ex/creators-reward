export type Money = {amount: string; currency: string};

// A site is `pending` from the moment it is added until a check succeeds
// (`verified`) or a check runs and the tag is absent (`failed`). `failed` is a
// recoverable state: install the tag and check again.
export type VerificationStatus = "pending" | "verified" | "failed";

export type Website = {
    id: string;
    domain: string;
    title: string;
    addedAt: string;
    status: VerificationStatus;
    // The value the owner pastes into the meta tag. Per-site, not per-account,
    // so one leaked token cannot be used to claim another site.
    token: string;
    lastCheckedAt: string | null;
};

type MonthlyStat = {
    month: string;
    views: number;
    viewers: number;
    earned: Money;
};

type PageStat = {
    url: string;
    title: string;
    views: number;
    earned: Money;
};

// Creator-side statistics are aggregates only. The per-view detail a viewer sees
// on their own dashboard is encrypted with the viewer's key, so nothing here can
// be traced back to an individual reader: `viewers` is a count, never a list.
export type WebsiteStats = {
    views: number;
    viewers: number;
    earned: Money;
    months: MonthlyStat[];
    topPages: PageStat[];
};

export type BankAccount = {
    holder: string;
    iban: string;
    bic: string;
};

type PayoutStatus = "paid" | "pending";

export type Payout = {
    id: string;
    period: string;
    amount: Money;
    status: PayoutStatus;
    paidAt: string | null;
};

// Mirrors the public profile served by GET /api/creators/{id} — the shape the
// viewer SPA renders on its creator page — plus the contact details that stay
// private to the portal.
export type CreatorProfile = {
    id: string;
    title: string;
    description: string;
    homepage: string;
    contactEmail: string;
};
