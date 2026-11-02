export type Money = {amount: string; currency: string};

export type Creator = {id: string; title: string};

export type CreatorProfile = Creator & {description: string};

type Article = {title: string; url: string};

// `creator` and `article` are not sent in the clear by the backend: they live
// inside the encrypted `payload` of GET /dashboard/views and are unsealed
// in the browser. They are flattened here because every consumer needs them
// decrypted, while only `at` and `cost` are ever readable server-side.
export type ViewItem = {
    id: string;
    at: string;
    cost: Money;
    creator: Creator;
    article: Article;
};

// Like ViewItem: everything but `id` lives inside the encrypted `payload` of
// GET /dashboard/enabled-donations and is unsealed in the browser.
// Decrypted in the browser from what the events service sealed to the
// viewer's key. The site (an origin) stands for the creator until creators
// register their sites.
export type EnabledDonation = {
    site: string;
    enabledAt: string;
    // Every stored record for this site. Normally one; more when the iframe
    // lost its per-site storage and the viewer enabled donations again.
    recordIds: string[];
};
