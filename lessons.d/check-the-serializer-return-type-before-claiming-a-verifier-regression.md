# Check the serializer return type before claiming a verifier regression

The first float-array runtime control treated serialize_jvm_class_file as a
Result and called unwrap, but that API returns Vec<u8>. Its compiler failure
is preserved and proves nothing about JVM descriptors or verification. Remove
the incorrect unwrap and rerun the unchanged behavioral control. Only an
executed JVM verifier failure is the runtime red evidence for this repair.

The next attempt failed with ClassFormatError because the injected launcher's
descriptor had not been registered as a UTF8 constant-pool entry. That failure
also does not prove the float-array defect. Register the launcher descriptor;
then verify the valid program and a descriptor-only [F to [I negative control
in separate real JVM processes, requiring VerifyError for the negative case.
