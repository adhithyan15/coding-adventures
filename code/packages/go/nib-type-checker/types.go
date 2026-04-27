package nibtypechecker

type NibType string

const (
	TypeU4   NibType = "u4"
	TypeU8   NibType = "u8"
	TypeBCD  NibType = "bcd"
	TypeBool NibType = "bool"
)

func ParseTypeName(name string) (NibType, bool) {
	switch name {
	case "u4":
		return TypeU4, true
	case "u8":
		return TypeU8, true
	case "bcd":
		return TypeBCD, true
	case "bool":
		return TypeBool, true
	default:
		return "", false
	}
}

func TypesAreCompatible(lhs NibType, rhs NibType) bool {
	return lhs == rhs
}

func IsBCDOpAllowed(operator string) bool {
	return operator == "+%" || operator == "-"
}

func IsNumeric(value NibType) bool {
	return value == TypeU4 || value == TypeU8 || value == TypeBCD
}
