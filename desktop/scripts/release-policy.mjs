export const UPDATER_CREDENTIAL_FIELDS = [
  "updaterPublicKey",
  "updaterPrivateKey",
  "updaterPrivateKeyPassword",
];
export const APPLE_CREDENTIAL_FIELDS = [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
  "KEYCHAIN_PASSWORD",
];
export const WINDOWS_CREDENTIAL_FIELDS = [
  "WINDOWS_CERTIFICATE",
  "WINDOWS_CERTIFICATE_PASSWORD",
  "WINDOWS_TIMESTAMP_URL",
];
const STABLE_TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export function evaluateReleasePolicy(input) {
  const { tag, signingMode } = input;
  const version = stableVersion(tag);
  requireValues(input, UPDATER_CREDENTIAL_FIELDS);
  if (signingMode === "updater-only") {
    return {
      version,
      macosSigning: "adhoc",
      windowsSigning: "unsigned",
    };
  }
  if (signingMode === "platform-signed") {
    requireCredentialSet(
      "Apple",
      input.appleCredentials,
      APPLE_CREDENTIAL_FIELDS,
    );
    requireCredentialSet(
      "Windows",
      input.windowsCredentials,
      WINDOWS_CREDENTIAL_FIELDS,
    );
    return {
      version,
      macosSigning: "developer-id",
      windowsSigning: "authenticode",
    };
  }
  throw new Error(
    `invalid Desktop signing mode ${signingMode}; expected updater-only or platform-signed`,
  );
}

function stableVersion(tag) {
  if (!STABLE_TAG.test(tag)) {
    throw new Error(`Desktop auto-update requires a stable vX.Y.Z tag: ${tag}`);
  }
  return tag.slice(1);
}

function requireValues(values, fields) {
  const missing = missingFields(values, fields);
  if (missing.length > 0) {
    throw new Error(`Desktop release credentials are missing: ${missing.join(" ")}`);
  }
}

function requireCredentialSet(label, values = {}, fields) {
  const missing = missingFields(values, fields);
  if (missing.length > 0) {
    throw new Error(`${label} signing credentials are missing: ${missing.join(" ")}`);
  }
}

function missingFields(values, fields) {
  return fields.filter((field) => !values[field]?.trim());
}
