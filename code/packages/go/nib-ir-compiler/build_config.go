package nibircompiler

type BuildConfig struct {
	InsertDebugComments    bool
	CopyParametersToLocals bool
	LocalRegisterBase      int
}

func DebugConfig() BuildConfig {
	return BuildConfig{InsertDebugComments: true}
}

func ReleaseConfig() BuildConfig {
	return BuildConfig{InsertDebugComments: false}
}

func CallSafeConfig() BuildConfig {
	return BuildConfig{
		CopyParametersToLocals: true,
		LocalRegisterBase:      defaultLocalRegisterBase,
	}
}
