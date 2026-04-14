export interface BuildConfig {
  readonly insertDebugComments: boolean;
}

export function debugConfig(): BuildConfig {
  return { insertDebugComments: true };
}

export function releaseConfig(): BuildConfig {
  return { insertDebugComments: false };
}
