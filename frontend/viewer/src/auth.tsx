import {createContext} from "preact";
import type {ComponentChildren} from "preact";
import {useCallback, useContext, useEffect, useMemo, useState} from "preact/hooks";
import {fetchSession, logout as logoutApi} from "./authApi";
import {clearViewerPrivateKey, loadViewerPrivateKey} from "./viewerKeys";

type AuthValue = {
    email: string | null;
    isLoggedIn: boolean;
    isLoading: boolean;
    login: (email: string) => void;
    logout: () => void;
};

// The private key lives exactly as long as the session. A key left over from a
// session that expired without a logout is dropped; a session the key did not
// survive into (site data cleared) can read nothing, so it is ended and the
// viewer logs in again, which restores the key.
async function sessionWithKey(): Promise<string | null> {
    const session = await fetchSession();
    if (session === null) {
        await clearViewerPrivateKey();
        return null;
    }
    if (await loadViewerPrivateKey()) return session.email;
    await logoutApi();
    return null;
}

const AuthContext = createContext<AuthValue | null>(null);

export function AuthProvider(props: {children: ComponentChildren; initialEmail?: string}) {
    const [email, setEmail] = useState<string | null>(props.initialEmail ?? null);
    const [isLoading, setIsLoading] = useState<boolean>(props.initialEmail === undefined);

    useEffect(() => {
        if (props.initialEmail !== undefined) return;
        sessionWithKey()
            .then(setEmail)
            .catch(() => {
                // network error — stay logged out, don't crash
                setEmail(null);
            })
            .finally(() => {
                setIsLoading(false);
            });
    }, [props.initialEmail]);

    const login = useCallback((newEmail: string) => {
        setEmail(newEmail);
    }, []);

    const logout = useCallback(() => {
        logoutApi().catch(() => {
            // ignore logout errors — we clear state regardless
        });
        clearViewerPrivateKey().catch(() => {
            // nothing stored, or storage unavailable: nothing to clear
        });
        setEmail(null);
    }, []);

    const value = useMemo<AuthValue>(
        () => ({
            email,
            isLoggedIn: email !== null,
            isLoading,
            login,
            logout,
        }),
        [email, isLoading, login, logout],
    );

    return <AuthContext.Provider value={value}>{props.children}</AuthContext.Provider>;
}

export function useAuth(): AuthValue {
    const value = useContext(AuthContext);
    if (value === null) {
        throw new Error("useAuth must be used inside <AuthProvider>");
    }
    return value;
}
