import {useId, useState} from "preact/hooks";

type CopyState = "idle" | "copied" | "failed";

export function CopyableCode(props: {label: string; value: string}) {
    const id = useId();
    const [state, setState] = useState<CopyState>("idle");

    function onCopy() {
        try {
            navigator.clipboard.writeText(props.value).then(
                () => setState("copied"),
                () => setState("failed"),
            );
        } catch {
            // No clipboard permission, or no clipboard at all (older browsers,
            // and jsdom under test). Fall back to telling the user to copy by hand.
            setState("failed");
        }
    }

    return (
        <div className="flex flex-col gap-1">
            <label className="label" htmlFor={id}>{props.label}</label>
            <div className="join w-full">
                <input
                    id={id}
                    className="input join-item w-full font-mono text-xs"
                    readOnly
                    value={props.value}
                />
                <button type="button" className="btn join-item" onClick={onCopy}>
                    Copy
                </button>
            </div>
            {state === "copied" && (
                <p className="label text-success">Copied to clipboard.</p>
            )}
            {state === "failed" && (
                <p className="label">Select the text above and copy it manually.</p>
            )}
        </div>
    );
}
