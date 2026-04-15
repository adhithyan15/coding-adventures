package nibircompiler

type BuildConfig struct {
	InsertDebugComments bool
}

func DebugConfig() BuildConfig {
	return BuildConfig{InsertDebugComments: true}
}

func ReleaseConfig() BuildConfig {
	return BuildConfig{InsertDebugComments: false}
}
