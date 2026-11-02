export const Routes = {
    Home: "/",
    Login: "/login",
    Register: "/register",
    Dashboard: "/dashboard",
    EnabledDonations: "/enabled-donations",
    CreatorInfo: creatorInfoRoute(":id"),
} as const;

export function creatorInfoRoute(id: string): string {
    return `/creator/${id}`
}
