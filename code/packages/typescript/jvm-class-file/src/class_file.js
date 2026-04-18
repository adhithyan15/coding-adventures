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
const CONSTANT_UTF8 = 1;
const CONSTANT_INTEGER = 3;
const CONSTANT_LONG = 5;
const CONSTANT_DOUBLE = 6;
const CONSTANT_CLASS = 7;
const CONSTANT_STRING = 8;
const CONSTANT_FIELDREF = 9;
const CONSTANT_METHODREF = 10;
const CONSTANT_NAME_AND_TYPE = 12;
export const ACC_PUBLIC = 0x0001;
export const ACC_STATIC = 0x0008;
export const ACC_SUPER = 0x0020;
export class ClassFileFormatError extends Error {
    constructor(message) {
        super(message);
        this.name = "ClassFileFormatError";
    }
}
class ParsedJVMMethodInfo {
    accessFlags;
    name;
    descriptor;
    attributes;
    constructor(accessFlags, name, descriptor, attributes) {
        this.accessFlags = accessFlags;
        this.name = name;
        this.descriptor = descriptor;
        this.attributes = attributes;
    }
    codeAttribute() {
        for (const attribute of this.attributes) {
            if ("maxStack" in attribute) {
                return attribute;
            }
        }
        return null;
    }
}
class ParsedJVMClassFile {
    version;
    accessFlags;
    thisClassName;
    superClassName;
    constantPool;
    methods;
    constructor(version, accessFlags, thisClassName, superClassName, constantPool, methods) {
        this.version = version;
        this.accessFlags = accessFlags;
        this.thisClassName = thisClassName;
        this.superClassName = superClassName;
        this.constantPool = constantPool;
        this.methods = methods;
    }
    getUtf8(index) {
        const entry = this.entry(index);
        if (entry?.kind !== "Utf8") {
            throw new ClassFileFormatError(`Constant pool entry ${index} is not a UTF-8 string`);
        }
        return entry.value;
    }
    resolveClassName(index) {
        const entry = this.entry(index);
        if (entry?.kind !== "Class") {
            throw new ClassFileFormatError(`Constant pool entry ${index} is not a Class entry`);
        }
        return this.getUtf8(entry.nameIndex);
    }
    resolveNameAndType(index) {
        const entry = this.entry(index);
        if (entry?.kind !== "NameAndType") {
            throw new ClassFileFormatError(`Constant pool entry ${index} is not a NameAndType entry`);
        }
        return [this.getUtf8(entry.nameIndex), this.getUtf8(entry.descriptorIndex)];
    }
    resolveConstant(index) {
        const entry = this.entry(index);
        switch (entry?.kind) {
            case "Utf8":
                return entry.value;
            case "Integer":
                return entry.value;
            case "Long":
                return entry.value;
            case "Double":
                return entry.value;
            case "String":
                return this.getUtf8(entry.stringIndex);
            default:
                throw new ClassFileFormatError(`Constant pool entry ${index} is not a loadable constant`);
        }
    }
    resolveFieldref(index) {
        const entry = this.entry(index);
        if (entry?.kind !== "Fieldref") {
            throw new ClassFileFormatError(`Constant pool entry ${index} is not a Fieldref entry`);
        }
        const [name, descriptor] = this.resolveNameAndType(entry.nameAndTypeIndex);
        return {
            className: this.resolveClassName(entry.classIndex),
            name,
            descriptor,
        };
    }
    resolveMethodref(index) {
        const entry = this.entry(index);
        if (entry?.kind !== "Methodref") {
            throw new ClassFileFormatError(`Constant pool entry ${index} is not a Methodref entry`);
        }
        const [name, descriptor] = this.resolveNameAndType(entry.nameAndTypeIndex);
        return {
            className: this.resolveClassName(entry.classIndex),
            name,
            descriptor,
        };
    }
    ldcConstants() {
        const lookup = new Map();
        for (let index = 1; index < this.constantPool.length; index += 1) {
            const entry = this.constantPool[index];
            if (entry?.kind === "Integer") {
                lookup.set(index, entry.value);
            }
            else if (entry?.kind === "String") {
                lookup.set(index, this.getUtf8(entry.stringIndex));
            }
        }
        return lookup;
    }
    findMethod(name, descriptor) {
        for (const method of this.methods) {
            if (method.name !== name) {
                continue;
            }
            if (descriptor !== undefined && method.descriptor !== descriptor) {
                continue;
            }
            return method;
        }
        return null;
    }
    entry(index) {
        if (!Number.isInteger(index) || index <= 0 || index >= this.constantPool.length) {
            throw new ClassFileFormatError(`Constant pool index ${index} is out of range`);
        }
        const entry = this.constantPool[index];
        if (entry === null) {
            throw new ClassFileFormatError(`Constant pool index ${index} points at a reserved wide slot`);
        }
        return entry;
    }
}
class ClassReader {
    data;
    offset = 0;
    constructor(data) {
        this.data = data;
    }
    readU1() {
        return this.readExact(1)[0];
    }
    readU2() {
        const view = this.readExact(2);
        return (view[0] << 8) | view[1];
    }
    readU4() {
        const view = this.readExact(4);
        return (((view[0] << 24) >>> 0) |
            (view[1] << 16) |
            (view[2] << 8) |
            view[3]) >>> 0;
    }
    readBytes(length) {
        if (!Number.isInteger(length) || length < 0) {
            throw new ClassFileFormatError(`Invalid byte length ${length}`);
        }
        if (length > this.remaining()) {
            throw new ClassFileFormatError(`Unexpected end of class file while reading ${length} bytes`);
        }
        const start = this.offset;
        this.offset += length;
        return this.data.slice(start, start + length);
    }
    remaining() {
        return this.data.length - this.offset;
    }
    atEnd() {
        return this.offset === this.data.length;
    }
    readExact(length) {
        return this.readBytes(length);
    }
}
class ConstantPoolBuilder {
    entries = [];
    indices = new Map();
    utf8(value) {
        const payload = new TextEncoder().encode(value);
        return this.add(`Utf8:${value}`, concatBytes(u1(CONSTANT_UTF8), u2(payload.length), payload));
    }
    integer(value) {
        const buffer = new ArrayBuffer(4);
        new DataView(buffer).setInt32(0, value, false);
        return this.add(`Integer:${value}`, concatBytes(u1(CONSTANT_INTEGER), new Uint8Array(buffer)));
    }
    classRef(internalName) {
        return this.add(`Class:${internalName}`, concatBytes(u1(CONSTANT_CLASS), u2(this.utf8(internalName))));
    }
    string(value) {
        return this.add(`String:${value}`, concatBytes(u1(CONSTANT_STRING), u2(this.utf8(value))));
    }
    nameAndType(name, descriptor) {
        return this.add(`NameAndType:${name}:${descriptor}`, concatBytes(u1(CONSTANT_NAME_AND_TYPE), u2(this.utf8(name)), u2(this.utf8(descriptor))));
    }
    fieldRef(owner, name, descriptor) {
        return this.add(`Fieldref:${owner}:${name}:${descriptor}`, concatBytes(u1(CONSTANT_FIELDREF), u2(this.classRef(owner)), u2(this.nameAndType(name, descriptor))));
    }
    methodRef(owner, name, descriptor) {
        return this.add(`Methodref:${owner}:${name}:${descriptor}`, concatBytes(u1(CONSTANT_METHODREF), u2(this.classRef(owner)), u2(this.nameAndType(name, descriptor))));
    }
    entriesBytes() {
        return concatBytes(...this.entries);
    }
    size() {
        return this.entries.length + 1;
    }
    add(key, payload) {
        const existing = this.indices.get(key);
        if (existing !== undefined) {
            return existing;
        }
        this.entries.push(payload);
        const index = this.entries.length;
        this.indices.set(key, index);
        return index;
    }
}
export function parseClassFile(data) {
    const reader = new ClassReader(data);
    if (reader.readU4() !== 0xcafebabe) {
        throw new ClassFileFormatError("Expected 0xCAFEBABE class-file magic");
    }
    const minor = reader.readU2();
    const major = reader.readU2();
    const constantPoolCount = reader.readU2();
    const constantPool = [null];
    for (let index = 1; index < constantPoolCount; index += 1) {
        const tag = reader.readU1();
        switch (tag) {
            case CONSTANT_UTF8: {
                const length = reader.readU2();
                const value = new TextDecoder("utf-8", { fatal: true }).decode(reader.readBytes(length));
                constantPool.push({ kind: "Utf8", value });
                break;
            }
            case CONSTANT_INTEGER: {
                const bytes = reader.readBytes(4);
                const value = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getInt32(0, false);
                constantPool.push({ kind: "Integer", value });
                break;
            }
            case CONSTANT_LONG: {
                const high = BigInt(reader.readU4());
                const low = BigInt(reader.readU4());
                constantPool.push({ kind: "Long", value: BigInt.asIntN(64, (high << 32n) | low) });
                constantPool.push(null);
                index += 1;
                break;
            }
            case CONSTANT_DOUBLE: {
                const bytes = reader.readBytes(8);
                const value = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat64(0, false);
                constantPool.push({ kind: "Double", value });
                constantPool.push(null);
                index += 1;
                break;
            }
            case CONSTANT_CLASS:
                constantPool.push({ kind: "Class", nameIndex: reader.readU2() });
                break;
            case CONSTANT_STRING:
                constantPool.push({ kind: "String", stringIndex: reader.readU2() });
                break;
            case CONSTANT_NAME_AND_TYPE:
                constantPool.push({
                    kind: "NameAndType",
                    nameIndex: reader.readU2(),
                    descriptorIndex: reader.readU2(),
                });
                break;
            case CONSTANT_FIELDREF:
                constantPool.push({
                    kind: "Fieldref",
                    classIndex: reader.readU2(),
                    nameAndTypeIndex: reader.readU2(),
                });
                break;
            case CONSTANT_METHODREF:
                constantPool.push({
                    kind: "Methodref",
                    classIndex: reader.readU2(),
                    nameAndTypeIndex: reader.readU2(),
                });
                break;
            default:
                throw new ClassFileFormatError(`Unsupported constant pool tag ${tag}`);
        }
    }
    const accessFlags = reader.readU2();
    const thisClassIndex = reader.readU2();
    const superClassIndex = reader.readU2();
    const interfaceCount = reader.readU2();
    reader.readBytes(interfaceCount * 2);
    const fieldCount = reader.readU2();
    for (let index = 0; index < fieldCount; index += 1) {
        skipMember(reader);
    }
    const methods = [];
    const methodCount = reader.readU2();
    for (let index = 0; index < methodCount; index += 1) {
        methods.push(parseMethod(reader, constantPool));
    }
    const classAttributeCount = reader.readU2();
    for (let index = 0; index < classAttributeCount; index += 1) {
        skipAttribute(reader);
    }
    if (!reader.atEnd()) {
        throw new ClassFileFormatError("Trailing bytes after end of class file");
    }
    const parsed = new ParsedJVMClassFile({ major, minor }, accessFlags, resolveClassName(constantPool, thisClassIndex), superClassIndex === 0 ? null : resolveClassName(constantPool, superClassIndex), Object.freeze([...constantPool]), Object.freeze(methods));
    return parsed;
}
export function buildMinimalClassFile(params) {
    const majorVersion = params.majorVersion ?? 49;
    const minorVersion = params.minorVersion ?? 0;
    const classAccessFlags = params.classAccessFlags ?? (ACC_PUBLIC | ACC_SUPER);
    const methodAccessFlags = params.methodAccessFlags ?? (ACC_PUBLIC | ACC_STATIC);
    const superClassName = params.superClassName ?? "java/lang/Object";
    const pool = new ConstantPoolBuilder();
    const thisClassIndex = pool.classRef(params.className);
    const superClassIndex = pool.classRef(superClassName);
    const methodNameIndex = pool.utf8(params.methodName);
    const descriptorIndex = pool.utf8(params.descriptor);
    const codeNameIndex = pool.utf8("Code");
    for (const value of params.constants ?? []) {
        pool.integer(value);
    }
    const codeAttribute = concatBytes(u2(codeNameIndex), u4(12 + params.code.length), u2(params.maxStack), u2(params.maxLocals), u4(params.code.length), params.code, u2(0), u2(0));
    const methodInfo = concatBytes(u2(methodAccessFlags), u2(methodNameIndex), u2(descriptorIndex), u2(1), codeAttribute);
    return concatBytes(u4(0xcafebabe), u2(minorVersion), u2(majorVersion), u2(pool.size()), pool.entriesBytes(), u2(classAccessFlags), u2(thisClassIndex), u2(superClassIndex), u2(0), u2(0), u2(1), methodInfo, u2(0));
}
function skipMember(reader) {
    reader.readU2();
    reader.readU2();
    reader.readU2();
    const attributeCount = reader.readU2();
    for (let index = 0; index < attributeCount; index += 1) {
        skipAttribute(reader);
    }
}
function skipAttribute(reader) {
    reader.readU2();
    const length = reader.readU4();
    reader.readBytes(length);
}
function parseMethod(reader, constantPool) {
    const accessFlags = reader.readU2();
    const nameIndex = reader.readU2();
    const descriptorIndex = reader.readU2();
    const attributeCount = reader.readU2();
    const attributes = [];
    for (let index = 0; index < attributeCount; index += 1) {
        attributes.push(parseAttribute(reader, constantPool, false));
    }
    return new ParsedJVMMethodInfo(accessFlags, getUtf8(constantPool, nameIndex), getUtf8(constantPool, descriptorIndex), Object.freeze(attributes));
}
function parseAttribute(reader, constantPool, insideCode) {
    const nameIndex = reader.readU2();
    const length = reader.readU4();
    const name = getUtf8(constantPool, nameIndex);
    if (name === "Code" && !insideCode) {
        const attributeBytes = new ClassReader(reader.readBytes(length));
        const maxStack = attributeBytes.readU2();
        const maxLocals = attributeBytes.readU2();
        const codeLength = attributeBytes.readU4();
        const code = attributeBytes.readBytes(codeLength);
        const exceptionTableLength = attributeBytes.readU2();
        attributeBytes.readBytes(exceptionTableLength * 8);
        const nestedCount = attributeBytes.readU2();
        const nestedAttributes = [];
        for (let index = 0; index < nestedCount; index += 1) {
            const nested = parseAttribute(attributeBytes, constantPool, true);
            if ("maxStack" in nested) {
                throw new ClassFileFormatError("Nested Code attributes are not supported");
            }
            nestedAttributes.push(nested);
        }
        if (!attributeBytes.atEnd()) {
            throw new ClassFileFormatError("Trailing bytes inside Code attribute");
        }
        return {
            name,
            maxStack,
            maxLocals,
            code,
            nestedAttributes: Object.freeze(nestedAttributes),
        };
    }
    return {
        name,
        info: reader.readBytes(length),
    };
}
function getUtf8(constantPool, index) {
    const entry = constantPool[index];
    if (entry?.kind !== "Utf8") {
        throw new ClassFileFormatError(`Constant pool entry ${index} is not a UTF-8 string`);
    }
    return entry.value;
}
function resolveClassName(constantPool, index) {
    const entry = constantPool[index];
    if (entry?.kind !== "Class") {
        throw new ClassFileFormatError(`Constant pool entry ${index} is not a Class entry`);
    }
    return getUtf8(constantPool, entry.nameIndex);
}
function u1(value) {
    if (!Number.isInteger(value) || value < 0 || value > 0xff) {
        throw new Error(`u1 out of range: ${value}`);
    }
    return Uint8Array.of(value);
}
function u2(value) {
    if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
        throw new Error(`u2 out of range: ${value}`);
    }
    return Uint8Array.of((value >>> 8) & 0xff, value & 0xff);
}
function u4(value) {
    if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
        throw new Error(`u4 out of range: ${value}`);
    }
    return Uint8Array.of((value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff);
}
function concatBytes(...parts) {
    const size = parts.reduce((sum, part) => sum + part.length, 0);
    const result = new Uint8Array(size);
    let offset = 0;
    for (const part of parts) {
        result.set(part, offset);
        offset += part.length;
    }
    return result;
}
//# sourceMappingURL=class_file.js.map