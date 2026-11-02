import type {CreatorProfile, Money, ViewItem} from "./types";

// Stand-in for the backend. Every function is
// synchronous and pure over its arguments: no fetch, no module-level mutation,
// so pages own their copy of the data and tests never leak state into each other.

const EUR = (amount: string): Money => ({amount, currency: "EUR"});

const creatorProfiles: Record<string, CreatorProfile> = {
    "1": {
        id: "1",
        title: "Harbour Gazette",
        description:
            "Independent local newsroom covering investigations, politics and city hall.",
    },
    "2": {
        id: "2",
        title: "Northern Post",
        description:
            "Daily reporting on national and world news, politics, culture and sport.",
    },
    "3": {
        id: "3",
        title: "Wikipedia",
        description:
            "Collaboratively written encyclopaedia. Rewards are shared with article authors.",
    },
};

export const initialViews: ViewItem[] = [
    {
        id: "8f1d0c4e-1a2b-4c3d-9e5f-000000000001",
        at: "2023-11-30T12:00:00Z",
        cost: EUR("0.01"),
        creator: {id: "1", title: "Harbour Gazette"},
        article: {
            url: "https://harbour-gazette.example/investigations/water-contracts",
            title:
                "How a small council lost track of its own water contracts",
        },
    },
    {
        id: "8f1d0c4e-1a2b-4c3d-9e5f-000000000002",
        at: "2023-11-30T12:15:00Z",
        cost: EUR("0.02"),
        creator: {id: "2", title: "Northern Post"},
        article: {
            url: "https://northern-post.example/news/2023/nov/30/rail-strike-commuters",
            title:
                "Rail strike: what changes for commuters next week",
        },
    },
    {
        id: "8f1d0c4e-1a2b-4c3d-9e5f-000000000003",
        at: "2023-12-02T09:30:00Z",
        cost: EUR("0.03"),
        creator: {id: "2", title: "Northern Post"},
        article: {
            url: "https://northern-post.example/environment/2023/dec/02/climate-summit-renewables",
            title: "Climate summit: nearly 120 countries back tripling renewable energy by 2030",
        },
    },
    {
        id: "8f1d0c4e-1a2b-4c3d-9e5f-000000000004",
        at: "2023-12-14T18:45:00Z",
        cost: EUR("0.01"),
        creator: {id: "3", title: "Wikipedia"},
        article: {
            url: "https://en.wikipedia.org/wiki/Micropayment",
            title: "Micropayment",
        },
    },
];

export function getCreator(id: string): CreatorProfile | undefined {
    return creatorProfiles[id];
}

export function formatMoney(money: Money): string {
    return `${money.amount} ${money.currency}`;
}

export function totalCost(views: readonly ViewItem[]): string {
    // Amounts are decimal strings on the wire to avoid float drift; sum in
    // integer cents and re-render, rather than adding the floats directly.
    const cents = views.reduce((sum, v) => sum + Math.round(Number(v.cost.amount) * 100), 0);
    return `${(cents / 100).toFixed(2)} EUR`;
}

export function formatDateTime(iso: string): string {
    const date = new Date(iso);
    return `${date.toLocaleDateString("en-GB")} ${date.toLocaleTimeString("en-GB", {
        hour: "2-digit",
        minute: "2-digit",
    })}`;
}

export function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString("en-GB");
}

export function monthKey(iso: string): string {
    return iso.slice(0, 7);
}

export function monthLabel(key: string): string {
    const [year, month] = key.split("-");
    if (year === undefined || month === undefined) return key;
    const date = new Date(Number(year), Number(month) - 1, 1);
    return date.toLocaleDateString("en-GB", {month: "short", year: "numeric"});
}
