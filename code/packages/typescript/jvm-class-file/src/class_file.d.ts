/**
 * Minimal JVM class-file support for the TypeScript JVM rollout.
 *
 * The full JVM class-file format is large. This package intentionally models
 * the subset our repository's compiler pipeline needs first:
 *
 *   class file
 *     -> constant pool
 *     -> methods
 *     -> Code attribute
 *
 * That is enough for the generic JVM backend to build parseable `.class` files
 * and for the source-language orchestrators to sanity-check their output.
 */
export declare const ACC_PUBLIC = 1;
export declare const ACC_STATIC = 8;
export declare const ACC_SUPER = 32;
export declare class ClassFileFormatError extends Error {
    constructor(message: string);
}
export interface JVMClassVersion {
    readonly major: number;
    readonly minor: number;
}
export interface JVMUtf8Info {
    readonly kind: "Utf8";
    readonly value: string;
}
export interface JVMIntegerInfo {
    readonly kind: "Integer";
    readonly value: number;
}
export interface JVMLongInfo {
    readonly kind: "Long";
    readonly value: bigint;
}
export interface JVMDoubleInfo {
    readonly kind: "Double";
    readonly value: number;
}
export interface JVMClassInfo {
    readonly kind: "Class";
    readonly nameIndex: number;
}
export interface JVMStringInfo {
    readonly kind: "String";
    readonly stringIndex: number;
}
export interface JVMNameAndTypeInfo {
    readonly kind: "NameAndType";
    readonly nameIndex: number;
    readonly descriptorIndex: number;
}
export interface JVMFieldrefInfo {
    readonly kind: "Fieldref";
    readonly classIndex: number;
    readonly nameAndTypeIndex: number;
}
export interface JVMMethodrefInfo {
    readonly kind: "Methodref";
    readonly classIndex: number;
    readonly nameAndTypeIndex: number;
}
export type JVMConstantPoolEntry = JVMUtf8Info | JVMIntegerInfo | JVMLongInfo | JVMDoubleInfo | JVMClassInfo | JVMStringInfo | JVMNameAndTypeInfo | JVMFieldrefInfo | JVMMethodrefInfo | null;
export interface JVMFieldReference {
    readonly className: string;
    readonly name: string;
    readonly descriptor: string;
}
export interface JVMMethodReference {
    readonly className: string;
    readonly name: string;
    readonly descriptor: string;
}
export interface JVMAttributeInfo {
    readonly name: string;
    readonly info: Uint8Array;
}
export interface JVMCodeAttribute {
    readonly name: string;
    readonly maxStack: number;
    readonly maxLocals: number;
    readonly code: Uint8Array;
    readonly nestedAttributes: readonly JVMAttributeInfo[];
}
export type JVMMethodAttribute = JVMAttributeInfo | JVMCodeAttribute;
export interface JVMMethodInfo {
    readonly accessFlags: number;
    readonly name: string;
    readonly descriptor: string;
    readonly attributes: readonly JVMMethodAttribute[];
    codeAttribute(): JVMCodeAttribute | null;
}
export interface JVMClassFile {
    readonly version: JVMClassVersion;
    readonly accessFlags: number;
    readonly thisClassName: string;
    readonly superClassName: string | null;
    readonly constantPool: readonly JVMConstantPoolEntry[];
    readonly methods: readonly JVMMethodInfo[];
    getUtf8(index: number): string;
    resolveClassName(index: number): string;
    resolveNameAndType(index: number): readonly [string, string];
    resolveConstant(index: number): number | bigint | string;
    resolveFieldref(index: number): JVMFieldReference;
    resolveMethodref(index: number): JVMMethodReference;
    ldcConstants(): ReadonlyMap<number, number | string>;
    findMethod(name: string, descriptor?: string): JVMMethodInfo | null;
}
export interface BuildMinimalClassFileParams {
    readonly className: string;
    readonly methodName: string;
    readonly descriptor: string;
    readonly code: Uint8Array;
    readonly maxStack: number;
    readonly maxLocals: number;
    readonly constants?: readonly number[];
    readonly majorVersion?: number;
    readonly minorVersion?: number;
    readonly classAccessFlags?: number;
    readonly methodAccessFlags?: number;
    readonly superClassName?: string;
}
export declare function parseClassFile(data: Uint8Array): JVMClassFile;
export declare function buildMinimalClassFile(params: BuildMinimalClassFileParams): Uint8Array;
//# sourceMappingURL=class_file.d.ts.map