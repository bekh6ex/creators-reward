import {createContext} from "preact";
import type {ComponentChildren} from "preact";
import {useCallback, useContext, useEffect, useMemo, useState} from "preact/hooks";
import {fetchSession, logout as logoutApi} from "./creatorApi";

type AuthValue = {
    email: string | null;
    isLoggedIn: boolean;
    // True until the session cookie has been checked, so a reload does not
    // bounce a logged-in creator to the home page.
    isLoading: boolean;
    // Records a session the API already started (register or login).
    login: (email: string) => void;
    logout: () => void;
};

const AuthContext = createContext<AuthValue | null>(null);

export function AuthProvider(props: {children: ComponentChildren; initialEmail?: string}) {
    const [email, setEmail] = useState<string | null>(props.initialEmail ?? null);
    const [isLoading, setIsLoading] = useState<boolean>(props.initialEmail === undefined);

    useEffect(() => {
        if (props.initialEmail !== undefined) return;
        fetchSession()
            .then(setEmail)
            .catch(() => {
                // Backend unreachable: stay logged out rather than crash.
                setEmail(null);
            })
            .finally(() => setIsLoading(false));
    }, [props.initialEmail]);

    const login = useCallback((next: string) => setEmail(next), []);

    const logout = useCallback(() => {
        logoutApi().catch(() => {
            // The local session ends regardless; the cookie expires on its own.
        });
        setEmail(null);
    }, []);

    const value = useMemo<AuthValue>(
        () => ({email, isLoggedIn: email !== null, isLoading, login, logout}),
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
