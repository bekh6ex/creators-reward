import type {BankAccount, CreatorProfile, Money, Payout, Website, WebsiteStats} from "./types";

// What the creator API does not serve yet: payouts, statistics, the profile, and
// password recovery are dummy data here, plus the pure helpers (validation,
// formatting). Accounts and websites are real and live in creatorApi.ts. Every
// function here is synchronous and pure over its arguments — no fetch.

const EUR = (amount: string): Money => ({amount, currency: "EUR"});

// ---------------------------------------------------------------- dummy data

// Keyed by website id. Only the test fixture's "1" has numbers: websites from the
// API have UUIDs, so they show empty statistics, which is also the truth — the
// pipeline does not count anything per website yet.
const statsByWebsite: Record<string, WebsiteStats> = {
    "1": {
        views: 18_432,
        viewers: 2_910,
        earned: EUR("284.60"),
        months: [
            {month: "2024-02", views: 5_120, viewers: 940, earned: EUR("78.20")},
            {month: "2024-03", views: 6_845, viewers: 1_105, earned: EUR("104.90")},
            {month: "2024-04", views: 6_467, viewers: 865, earned: EUR("101.50")},
        ],
        topPages: [
            {
                url: "https://daily-ledger.example/2024/04/water-contracts",
                title: "Who really owns the city's water contracts",
                views: 4_210,
                earned: EUR("64.80"),
            },
            {
                url: "https://daily-ledger.example/2024/03/budget-explainer",
                title: "The municipal budget, explained in nine charts",
                views: 3_180,
                earned: EUR("48.10"),
            },
            {
                url: "https://daily-ledger.example/2024/03/tram-delays",
                title: "Why the tram extension is four years late",
                views: 2_540,
                earned: EUR("37.90"),
            },
            {
                url: "https://daily-ledger.example/2024/02/housing-registry",
                title: "Inside the housing registry nobody audits",
                views: 1_905,
                earned: EUR("28.40"),
            },
        ],
    },
};

const EMPTY_STATS: WebsiteStats = {
    views: 0,
    viewers: 0,
    earned: EUR("0.00"),
    months: [],
    topPages: [],
};

export const initialPayouts: Payout[] = [
    {
        id: "p-2024-02",
        period: "2024-02",
        amount: EUR("78.20"),
        status: "paid",
        paidAt: "2024-03-05",
    },
    {
        id: "p-2024-03",
        period: "2024-03",
        amount: EUR("104.90"),
        status: "paid",
        paidAt: "2024-04-04",
    },
    {
        id: "p-2024-04",
        period: "2024-04",
        amount: EUR("101.50"),
        status: "pending",
        paidAt: null,
    },
];

// No account on file: adding one is the flow worth demonstrating, and it is the
// state every new creator starts in.
export const initialBankAccount: BankAccount | null = null;

export const initialProfile: CreatorProfile = {
    id: "1",
    title: "The Daily Ledger",
    description:
        "Independent local reporting on city government, transport and housing. Published weekly since 2019.",
    homepage: "https://daily-ledger.example",
    contactEmail: "editor@daily-ledger.example",
};

// ------------------------------------------------------------------ lookups

export function getWebsite(websites: readonly Website[], id: string): Website | undefined {
    return websites.find(website => website.id === id);
}

export function getWebsiteStats(id: string): WebsiteStats {
    return statsByWebsite[id] ?? EMPTY_STATS;
}

// ------------------------------------------------------------- verification

export function metaTag(token: string): string {
    return `<meta name="creators-reward-site-verification" content="${token}">`;
}

export function normaliseDomain(raw: string): string {
    return raw
        .trim()
        .toLowerCase()
        .replace(/^https?:\/\//, "")
        .replace(/^www\./, "")
        .replace(/[/?#].*$/, "");
}

export function validateDomain(domain: string): string | null {
    if (domain === "") {
        return "Enter the address of your website.";
    }
    if (!/^[a-z\d]([a-z\d-]*[a-z\d])?(\.[a-z\d]([a-z\d-]*[a-z\d])?)+$/.test(domain)) {
        return "That does not look like a domain name. Example: example.org";
    }
    return null;
}

// --------------------------------------------------------------------- IBAN

// The 27 EU member states, with the fixed IBAN length each one uses. Payouts
// start with the EU (other regions come later), so for now a non-EU IBAN is
// rejected even when its check digits are valid.
const EU_IBAN_LENGTH: Record<string, number> = {
    AT: 20, BE: 16, BG: 22, CY: 28, CZ: 24, DE: 22, DK: 18, EE: 20, ES: 24,
    FI: 18, FR: 27, GR: 27, HR: 21, HU: 28, IE: 22, IT: 27, LT: 20, LU: 20,
    LV: 21, MT: 31, NL: 18, PL: 28, PT: 25, RO: 24, SE: 24, SI: 19, SK: 24,
};

export function normaliseIban(raw: string): string {
    return raw.replaceAll(/\s/g, "").toUpperCase();
}

export function formatIban(iban: string): string {
    return (normaliseIban(iban).match(/.{1,4}/g) ?? []).join(" ");
}

// ISO 7064 MOD-97-10: move the first four characters to the end, replace letters
// with their position in the alphabet plus nine, and the whole number must leave
// a remainder of 1. Folded digit by digit to stay inside safe integer range.
function mod97(iban: string): number {
    const rearranged = iban.slice(4) + iban.slice(0, 4);
    let remainder = 0;
    for (const char of rearranged) {
        const value =
            char >= "A" && char <= "Z"
                ? String((char.codePointAt(0) ?? 0) - 55)
                : char;
        for (const digit of value) {
            remainder = (remainder * 10 + Number(digit)) % 97;
        }
    }
    return remainder;
}

export function validateIban(raw: string): string | null {
    const iban = normaliseIban(raw);
    if (iban === "") {
        return "Enter the IBAN of the account that should receive donations.";
    }
    if (!/^[A-Z]{2}\d{2}[A-Z\d]+$/.test(iban)) {
        return "An IBAN starts with two letters and two digits, for example DE89…";
    }
    const country = iban.slice(0, 2);
    const expected = EU_IBAN_LENGTH[country];
    if (expected === undefined) {
        return `${country} is not an EU country. Donations are paid to EU accounts only.`;
    }
    if (iban.length !== expected) {
        return `A ${country} IBAN is ${expected} characters long, this one is ${iban.length}.`;
    }
    if (mod97(iban) !== 1) {
        return "Those check digits do not match the account number. Re-type the IBAN.";
    }
    return null;
}

export function validateBic(raw: string): string | null {
    const bic = raw.trim().toUpperCase();
    if (bic === "") {
        return "Enter the BIC of your bank.";
    }
    if (!/^[A-Z]{6}[A-Z\d]{2}([A-Z\d]{3})?$/.test(bic)) {
        return "A BIC is 8 or 11 characters, for example COBADEFFXXX.";
    }
    return null;
}

// ---------------------------------------------------------------- formatting

export function isValidEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

export function formatMoney(money: Money): string {
    return `${money.amount} ${money.currency}`;
}

// Amounts are decimal strings on the wire to avoid float drift; sum in integer
// cents and re-render rather than adding the floats directly.
export function sumMoney(amounts: readonly Money[]): string {
    const cents = amounts.reduce(
        (sum, money) => sum + Math.round(Number(money.amount) * 100),
        0,
    );
    return `${(cents / 100).toFixed(2)} EUR`;
}

export function formatNumber(value: number): string {
    return value.toLocaleString("en-GB");
}

export function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString("en-GB");
}

export function formatDateTime(iso: string): string {
    const date = new Date(iso);
    return `${date.toLocaleDateString("en-GB")} ${date.toLocaleTimeString("en-GB", {
        hour: "2-digit",
        minute: "2-digit",
    })}`;
}

export function monthLabel(key: string): string {
    const [year, month] = key.split("-");
    if (year === undefined || month === undefined) return key;
    const date = new Date(Number(year), Number(month) - 1, 1);
    return date.toLocaleDateString("en-GB", {month: "short", year: "numeric"});
}
