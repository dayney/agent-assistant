export const DEFAULT_UPDATE_REPOSITORY = "dayney/agent-assistant";

export function githubReleasesBase(repository = DEFAULT_UPDATE_REPOSITORY) {
  const normalizedRepository = repository.trim();
  if (!/^[0-9A-Za-z_.-]+\/[0-9A-Za-z_.-]+$/.test(normalizedRepository)) {
    throw new Error(`invalid GitHub repository ${repository}`);
  }
  return `https://github.com/${normalizedRepository}/releases`;
}
