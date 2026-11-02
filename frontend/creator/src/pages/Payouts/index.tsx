import {useState} from "preact/hooks";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {usePortal} from "../../store";
import {
    formatDate,
    formatIban,
    formatMoney,
    monthLabel,
    normaliseIban,
    sumMoney,
    validateBic,
    validateIban,
} from "../../api";

export function Payouts() {
    const {bankAccount, saveBankAccount, payouts} = usePortal();
    const [editing, setEditing] = useState(bankAccount === null);
    const [holder, setHolder] = useState(bankAccount?.holder ?? "");
    const [iban, setIban] = useState(bankAccount === null ? "" : formatIban(bankAccount.iban));
    const [bic, setBic] = useState(bankAccount?.bic ?? "");
    const [error, setError] = useState<string | null>(null);
    const [saved, setSaved] = useState(false);

    const paid = payouts.filter(payout => payout.status === "paid");
    const pending = payouts.filter(payout => payout.status === "pending");

    function onSubmit(event: Event) {
        event.preventDefault();
        if (holder.trim() === "") {
            setError("Enter the name the account is held in.");
            return;
        }
        const ibanProblem = validateIban(iban);
        if (ibanProblem !== null) {
            setError(ibanProblem);
            return;
        }
        const bicProblem = validateBic(bic);
        if (bicProblem !== null) {
            setError(bicProblem);
            return;
        }
        setError(null);
        saveBankAccount({
            holder: holder.trim(),
            iban: normaliseIban(iban),
            bic: bic.trim().toUpperCase(),
        });
        setEditing(false);
        setSaved(true);
    }

    function onEdit() {
        setSaved(false);
        setEditing(true);
    }

    return (
        <AuthenticatedLayout>
            <div className="flex max-w-3xl flex-col gap-4">
                <h1 className="text-2xl font-semibold">Payouts</h1>

                <section className="card bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h2 className="card-title">Bank account</h2>

                        {bankAccount === null && (
                            <div role="alert" className="alert alert-warning">
                                <span>
                                    We have nowhere to send your money yet. Add an EU account
                                    to receive donations.
                                </span>
                            </div>
                        )}

                        {saved && (
                            <div role="alert" className="alert alert-success">
                                <span>Bank account saved.</span>
                            </div>
                        )}

                        {editing ? (
                            <form onSubmit={onSubmit} noValidate>
                                <fieldset className="fieldset">
                                    <label className="input w-full">
                                        <span className="label">Account holder</span>
                                        <input
                                            type="text"
                                            autocomplete="name"
                                            value={holder}
                                            onInput={event =>
                                                setHolder(event.currentTarget.value)
                                            }
                                        />
                                    </label>
                                    <label className="input w-full">
                                        <span className="label">IBAN</span>
                                        <input
                                            type="text"
                                            placeholder="DE89 3704 0044 0532 0130 00"
                                            value={iban}
                                            onInput={event =>
                                                setIban(event.currentTarget.value)
                                            }
                                        />
                                    </label>
                                    <label className="input w-full">
                                        <span className="label">BIC</span>
                                        <input
                                            type="text"
                                            placeholder="COBADEFFXXX"
                                            value={bic}
                                            onInput={event => setBic(event.currentTarget.value)}
                                        />
                                    </label>
                                    <p className="label">
                                        EU accounts only. Payouts are made in euro, once a
                                        month.
                                    </p>
                                </fieldset>

                                {error !== null && (
                                    <div role="alert" className="alert alert-error mt-4">
                                        <span>{error}</span>
                                    </div>
                                )}

                                <button type="submit" className="btn btn-primary mt-4">
                                    Save account
                                </button>
                            </form>
                        ) : (
                            bankAccount !== null && (
                                <>
                                    <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1">
                                        <dt className="text-base-content/70">Holder</dt>
                                        <dd>{bankAccount.holder}</dd>
                                        <dt className="text-base-content/70">IBAN</dt>
                                        <dd className="font-mono text-sm">
                                            {formatIban(bankAccount.iban)}
                                        </dd>
                                        <dt className="text-base-content/70">BIC</dt>
                                        <dd className="font-mono text-sm">{bankAccount.bic}</dd>
                                    </dl>
                                    <div className="card-actions mt-2">
                                        <button type="button" className="btn" onClick={onEdit}>
                                            Change account
                                        </button>
                                    </div>
                                </>
                            )
                        )}
                    </div>
                </section>

                <section className="flex flex-col gap-4">
                    <h2 className="text-xl font-semibold">History</h2>

                    <div className="stats stats-vertical w-full bg-base-200 sm:stats-horizontal sm:w-auto sm:self-start">
                        <div className="stat">
                            <div className="stat-title">Paid out</div>
                            <div className="stat-value text-2xl">
                                {sumMoney(paid.map(payout => payout.amount))}
                            </div>
                        </div>
                        <div className="stat">
                            <div className="stat-title">Next payout</div>
                            <div className="stat-value text-2xl">
                                {sumMoney(pending.map(payout => payout.amount))}
                            </div>
                        </div>
                    </div>

                    {payouts.length === 0 ? (
                        <div role="alert" className="alert">
                            <span>No payouts yet.</span>
                        </div>
                    ) : (
                        <div className="overflow-x-auto">
                            <table className="table">
                                <thead>
                                <tr>
                                    <th>Period</th>
                                    <th>Amount</th>
                                    <th>Status</th>
                                    <th>Paid on</th>
                                </tr>
                                </thead>
                                <tbody>
                                {payouts.map(payout => (
                                    <tr key={payout.id}>
                                        <td>{monthLabel(payout.period)}</td>
                                        <td>{formatMoney(payout.amount)}</td>
                                        <td>
                                            <span
                                                className={`badge ${
                                                    payout.status === "paid"
                                                        ? "badge-success"
                                                        : "badge-warning"
                                                }`}
                                            >
                                                {payout.status === "paid" ? "Paid" : "Pending"}
                                            </span>
                                        </td>
                                        <td>
                                            {payout.paidAt === null
                                                ? "—"
                                                : formatDate(payout.paidAt)}
                                        </td>
                                    </tr>
                                ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                </section>
            </div>
        </AuthenticatedLayout>
    );
}
