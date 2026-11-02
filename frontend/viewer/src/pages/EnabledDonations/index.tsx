import {useEffect, useState} from "preact/hooks";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {formatDate} from "../../api";
import {disableDonations, fetchEnabledDonations} from "../../enabledDonationsApi";
import type {EnabledDonation} from "../../types";

type State =
    | {status: "loading"}
    | {status: "failed"; message: string}
    | {status: "loaded"; enabledDonations: EnabledDonation[]};

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

export function EnabledDonations() {
    const [state, setState] = useState<State>({status: "loading"});
    const [disabling, setDisabling] = useState<string | null>(null);
    const [disableError, setDisableError] = useState<string | null>(null);

    useEffect(() => {
        let active = true;
        fetchEnabledDonations().then(
            enabledDonations => {
                if (active) setState({status: "loaded", enabledDonations});
            },
            (error: unknown) => {
                if (active) {
                    setState({status: "failed", message: errorMessage(error)});
                }
            },
        );
        return () => {
            active = false;
        };
    }, []);

    async function disable(donation: EnabledDonation) {
        setDisabling(donation.site);
        setDisableError(null);
        try {
            await disableDonations(donation);
            setState(current =>
                current.status === "loaded"
                    ? {
                          status: "loaded",
                          enabledDonations: current.enabledDonations.filter(
                              enabled => enabled.site !== donation.site,
                          ),
                      }
                    : current,
            );
        } catch (error) {
            setDisableError(errorMessage(error));
        } finally {
            setDisabling(null);
        }
    }

    return (
        <AuthenticatedLayout>
            {state.status === "loading" ? (
                <span className="loading loading-spinner" aria-label="Loading"/>
            ) : state.status === "failed" ? (
                <div role="alert" className="alert alert-error">
                    <span>{state.message}</span>
                </div>
            ) : state.enabledDonations.length === 0 ? (
                <div role="alert" className="alert">
                    <span>You have not enabled donations for any creator.</span>
                </div>
            ) : (
                <div className="overflow-x-auto">
                    {disableError && (
                        <div role="alert" className="alert alert-error mb-4">
                            <span>{disableError}</span>
                        </div>
                    )}
                    <table className="table">
                        <thead>
                        <tr>
                            <th>Creator</th>
                            <th>Enabled since</th>
                            <th>Actions</th>
                        </tr>
                        </thead>
                        <tbody>
                        {state.enabledDonations.map(enabled => (
                            <tr key={enabled.site}>
                                <td>
                                    <a
                                        className="link"
                                        href={enabled.site}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        {new URL(enabled.site).host}
                                    </a>
                                </td>
                                <td>{formatDate(enabled.enabledAt)}</td>
                                <td>
                                    <button
                                        type="button"
                                        className="btn btn-sm"
                                        disabled={disabling !== null}
                                        onClick={() => void disable(enabled)}
                                    >
                                        Disable donations
                                    </button>
                                </td>
                            </tr>
                        ))}
                        </tbody>
                    </table>
                </div>
            )}
        </AuthenticatedLayout>
    );
}
