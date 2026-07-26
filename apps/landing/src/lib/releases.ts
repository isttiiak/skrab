const REPO = 'isttiiak/skrab';

export type ReleaseAsset = {
  name: string;
  url: string;
  size: number;
};

export type Release = {
  tag: string;
  name: string;
  notes: string;
  publishedAt: string | null;
  assets: ReleaseAsset[];
};

type GithubAsset = { name: string; browser_download_url: string; size: number };
type GithubRelease = {
  tag_name: string;
  name: string | null;
  body: string | null;
  published_at: string | null;
  draft: boolean;
  prerelease: boolean;
  assets: GithubAsset[];
};

/**
 * Fetches published releases at build time.
 *
 * Returns an empty list rather than throwing when the repo has no releases yet, or
 * when the API is unreachable or rate-limited — a landing page that fails to build
 * because GitHub was briefly slow is worse than one that renders a "coming soon"
 * state. The release workflow triggers a rebuild, so the data refreshes on publish.
 */
export async function fetchReleases(): Promise<Release[]> {
  const token = import.meta.env.GITHUB_TOKEN;
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'skrab-landing',
  };
  if (token) headers.Authorization = `Bearer ${token}`;

  try {
    const response = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=20`, {
      headers,
    });
    if (!response.ok) {
      console.warn(`[releases] GitHub returned ${response.status}; rendering empty state`);
      return [];
    }

    const payload = (await response.json()) as GithubRelease[];
    return payload
      .filter((release) => !release.draft)
      .map((release) => ({
        tag: release.tag_name,
        name: release.name ?? release.tag_name,
        notes: release.body ?? '',
        publishedAt: release.published_at,
        assets: release.assets.map((asset) => ({
          name: asset.name,
          url: asset.browser_download_url,
          size: asset.size,
        })),
      }));
  } catch (error) {
    console.warn('[releases] could not reach GitHub; rendering empty state', error);
    return [];
  }
}

/** Picks the installer for a platform out of a release's asset list. */
export function assetFor(release: Release | undefined, platform: 'macos' | 'windows') {
  if (!release) return undefined;
  const matchers =
    platform === 'macos'
      ? [/\.dmg$/i, /\.app\.tar\.gz$/i]
      : [/\.msi$/i, /-setup\.exe$/i, /\.exe$/i];

  for (const matcher of matchers) {
    const hit = release.assets.find((asset) => matcher.test(asset.name));
    if (hit) return hit;
  }
  return undefined;
}

export function formatSize(bytes: number): string {
  if (bytes <= 0) return '';
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatDate(iso: string | null): string {
  if (!iso) return '';
  return new Date(iso).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}
