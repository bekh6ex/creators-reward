#!/usr/bin/env bash
#
# Generates a local CA + TLS certificate pair for *.crlocal domains
# (with localhost / 127.0.0.1 as extra SANs) for the dev stack.
# Non-production tooling, intentionally simple.
#
# The CA lives in ONE machine-wide store, not inside the checkout, so every
# worktree and clone shares it and the browser trust prompt happens once. Each
# checkout's dev/cert is a symlink to that store. The leaf is a wildcard for
# *.crlocal and ports are not part of a certificate, so one pair covers every
# worktree and every slot - there is nothing per-worktree to issue.
#
# Usage:  ./gen-cert.sh
#         CR_CERT_DIR=/some/path ./gen-cert.sh    # override the store location
#
# Output (in the store, surfaced through dev/cert):
#   ca.crt, ca.key        - short-lived local CA (~1 month)
#   cert.pem, key.pem     - server certificate pair

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
readonly SCRIPT_DIR

DEFAULT_STORE="${XDG_DATA_HOME:-$HOME/.local/share}/creators-reward/cert"
CERT_STORE="${CR_CERT_DIR:-$DEFAULT_STORE}"
readonly CERT_STORE
LINK_DIR="$SCRIPT_DIR/cert"
readonly LINK_DIR

CA_KEY="$CERT_STORE/ca.key"
CA_CRT="$CERT_STORE/ca.crt"
LEAF_KEY="$CERT_STORE/key.pem"
LEAF_CRT="$CERT_STORE/cert.pem"
CSR="$CERT_STORE/server.csr"

CANONICAL="crlocal"
WILDCARD="*.crlocal"
CA_DAYS=30
CA_REUSE_IF_LEFT=604800

mkdir -p "$CERT_STORE"

# Serialise generation: several worktrees may start at the same moment and would
# otherwise write this shared store concurrently.
exec 200>"$CERT_STORE/.lock"
flock 200

# An older layout kept real files in dev/cert. Move them into the shared store
# so a CA the browser already trusts keeps working instead of being replaced.
if [[ -d "$LINK_DIR" && ! -L "$LINK_DIR" ]]; then
    if [[ -f "$LINK_DIR/ca.crt" && ! -f "$CA_CRT" ]]; then
        echo "==> Adopting the existing CA from $LINK_DIR into the shared store"
        mv "$LINK_DIR"/* "$CERT_STORE"/ 2>/dev/null || true
    fi
    rm -rf "$LINK_DIR"
fi

if [[ -L "$LINK_DIR" && "$(readlink "$LINK_DIR")" != "$CERT_STORE" ]]; then
    rm -f "$LINK_DIR"
fi
[[ -e "$LINK_DIR" ]] || ln -s "$CERT_STORE" "$LINK_DIR"

# 1) Local CA. Short-lived (~1 month); the expiry date is part of its CN so it is
#    visible in the browser's certificate list. Reused across runs while it still
#    has a week of validity left; after that it is regenerated - re-import it!
CA_EXPIRY="$(date -d "+${CA_DAYS} days" +%Y-%m-%d)"
CA_CN="crlocal local dev CA until ${CA_EXPIRY}"
if [[ -f "$CA_CRT" && -f "$CA_KEY" ]] \
    && openssl x509 -in "$CA_CRT" -noout -checkend "$CA_REUSE_IF_LEFT" 2>/dev/null; then
    echo "==> Reusing existing CA: $CA_CRT"
    REUSED_CA=true
else
    [[ -f "$CA_CRT" ]] && echo "==> Existing CA expired or expiring within a week: regenerating"
    echo "==> Generating local CA (valid until ${CA_EXPIRY}): $CA_CRT"
    openssl req -x509 -newkey rsa:2048 -nodes -days "$CA_DAYS" \
        -subj "/CN=$CA_CN" \
        -keyout "$CA_KEY" -out "$CA_CRT" \
        -addext "basicConstraints=critical,CA:TRUE" \
        -addext "keyUsage=critical,keyCertSign,cRLSign"
    REUSED_CA=false
fi

# Reissuing the leaf against a CA that did not change would invalidate nothing
# and cost nothing, but it also gains nothing - skip it so concurrent worktrees
# are not swapping the file Caddy has open.
if [[ "$REUSED_CA" == true && -f "$LEAF_CRT" && -f "$LEAF_KEY" ]] \
    && openssl verify -CAfile "$CA_CRT" "$LEAF_CRT" >/dev/null 2>&1; then
    echo "==> Reusing existing server certificate: $LEAF_CRT"
else
    echo "==> Generating server certificate for $WILDCARD (+ localhost SANs)"
    openssl req -newkey rsa:2048 -nodes \
        -subj "/CN=$CANONICAL" \
        -keyout "$LEAF_KEY" -out "$CSR"

    #    Include explicit hostnames plus the wildcard so there are no surprises
    #    with OpenSSL/curl wildcard-matching quirks.
    openssl x509 -req -in "$CSR" \
        -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial \
        -days 825 -sha256 \
        -extfile <(printf '%s\n' \
            "basicConstraints=critical,CA:FALSE" \
            "keyUsage=critical,digitalSignature,keyEncipherment" \
            "extendedKeyUsage=serverAuth" \
            "subjectAltName=DNS:crlocal,DNS:$WILDCARD,DNS:website1.crlocal,DNS:website2.crlocal,DNS:app.crlocal,DNS:creator.crlocal,DNS:api.crlocal,DNS:localhost,IP:127.0.0.1") \
        -out "$LEAF_CRT"

    rm -f "$CSR"
fi

chmod 600 "$CA_KEY" "$LEAF_KEY"

echo
echo "Done. Shared certificate store: $CERT_STORE"
echo "     (surfaced in this checkout as $LINK_DIR -> $CERT_STORE)"
ls -l "$CERT_STORE"
echo
echo "Next steps:"
echo "  1. Trust the CA in your browser ONCE PER MACHINE - every worktree and"
echo "     clone reads this same file. The CA certificate is:"
echo "       $CA_CRT"
echo
echo "     Chrome / Chromium on Linux (system NSS db):"
echo "       mkdir -p \$HOME/.pki/nssdb   # if missing"
echo "       certutil -d sql:\$HOME/.pki/nssdb -A -t \"C,,\" -n crlocal-ca -i $CA_CRT"
echo
echo "     Firefox has its own store per profile - pick ONE:"
echo "       a) GUI: Settings -> Privacy & Security -> Certificates -> View Certificates"
echo "          -> Authorities -> Import... -> $CA_CRT"
echo "          -> tick \"Trust this CA to identify websites\""
echo "       b) Policy file (all profiles, persists) - /etc/firefox/policies/policies.json"
echo "          or <firefox-install-dir>/distribution/policies.json:"
echo "          { \"Certificates\": { \"Install\": [\"$CA_CRT\"] } }"
echo "       c) certutil into a specific profile (Firefox must be closed):"
echo "          certutil -d sql:\$HOME/.mozilla/firefox/<profile> -A -t \"C,,\" -n crlocal-ca -i $CA_CRT"
echo "       d) Or make Firefox read the system store: about:config ->"
echo "          security.enterprise_roots.enabled = true (then the Chrome command covers it)"
echo "     Without trusting the CA the browser rejects the certificate, and a"
echo "     rejected certificate kills the iframe (no click-through inside iframes)."
echo "  2. The certificates are used automatically by dev/start.sh"
echo
echo "  NOTE: the CA is valid for $CA_DAYS days (until $CA_EXPIRY)."
echo "        When it expires: re-run ./gen-cert.sh and import the new $CA_CRT"
echo "        into your browser again (the date in its name shows which one is live)."
