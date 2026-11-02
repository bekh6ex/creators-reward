export const Routes = {
    Home: "/",
    Login: "/login",
    Register: "/register",
    ForgotPassword: "/forgot-password",
    ResetPassword: resetPasswordRoute(":token"),
    Websites: "/websites",
    // Declared before Routes.Website in the Router: preact-iso takes the first
    // matching child, so ":id" would otherwise swallow "/websites/new".
    AddWebsite: "/websites/new",
    Website: websiteRoute(":id"),
    Payouts: "/payouts",
    Profile: "/profile",
} as const;

export function websiteRoute(id: string): string {
    return `/websites/${id}`;
}

export function resetPasswordRoute(token: string): string {
    return `/reset-password/${token}`;
}
