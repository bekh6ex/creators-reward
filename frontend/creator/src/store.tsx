import {createContext} from "preact";
import type {ComponentChildren} from "preact";
import {useContext, useEffect, useMemo, useState} from "preact/hooks";
import {initialBankAccount, initialPayouts, initialProfile} from "./api";
import {addWebsite as addWebsiteApi, listWebsites} from "./creatorApi";
import {useAuth} from "./auth";
import type {BankAccount, CreatorProfile, Payout, Website} from "./types";

// Portal-wide state. The viewer SPA gets away with per-page useState because its
// pages are independent, but here adding a site on one page has to be visible on
// another, so the data lives above the Router. Websites come from the creator
// API; the bank account, payouts and profile are still dummy data in memory.
type PortalValue = {
    websites: Website[];
    // Rejects with a CreatorApiError when the API refuses the site.
    addWebsite: (domain: string, title: string) => Promise<Website>;
    bankAccount: BankAccount | null;
    saveBankAccount: (account: BankAccount) => void;
    payouts: Payout[];
    profile: CreatorProfile;
    saveProfile: (profile: CreatorProfile) => void;
};

const PortalContext = createContext<PortalValue | null>(null);

export function PortalProvider(props: {
    children: ComponentChildren;
    // Seeds the list and skips loading it, for tests.
    websites?: Website[];
    bankAccount?: BankAccount | null;
}) {
    const {email} = useAuth();
    const [websites, setWebsites] = useState<Website[]>(props.websites ?? []);
    const [bankAccount, setBankAccount] = useState<BankAccount | null>(
        props.bankAccount ?? initialBankAccount,
    );
    const [profile, setProfile] = useState<CreatorProfile>(initialProfile);

    // Reload on every change of account; drop the list on logout.
    useEffect(() => {
        if (props.websites !== undefined) return;
        setWebsites([]);
        if (email === null) return;
        listWebsites()
            .then(setWebsites)
            .catch(() => {
                // Backend unreachable: show the empty list rather than crash.
            });
    }, [email, props.websites]);

    const value = useMemo<PortalValue>(
        () => ({
            websites,
            addWebsite: async (domain: string, title: string) => {
                const created = await addWebsiteApi(domain, title);
                setWebsites(current => [...current, created]);
                return created;
            },
            bankAccount,
            saveBankAccount: (account: BankAccount) => setBankAccount(account),
            payouts: initialPayouts,
            profile,
            saveProfile: (next: CreatorProfile) => setProfile(next),
        }),
        [websites, bankAccount, profile],
    );

    return <PortalContext.Provider value={value}>{props.children}</PortalContext.Provider>;
}

export function usePortal(): PortalValue {
    const value = useContext(PortalContext);
    if (value === null) {
        throw new Error("usePortal must be used inside <PortalProvider>");
    }
    return value;
}
