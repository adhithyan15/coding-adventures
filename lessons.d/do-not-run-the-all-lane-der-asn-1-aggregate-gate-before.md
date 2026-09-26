# Do not run the all-lane DER ASN.1 aggregate gate before every scaffold is complete

The DER ASN.1 aggregate coverage test intentionally requires complete source,
test, BUILD, capability, and fixture-consumer evidence for all fifteen
established lanes. Running it while the serial work item had only five complete
lanes produced expected failures against placeholder C# files. During partial
implementation, run the neutral fixture schema/oracle tests and each completed
package's native suite; reserve the aggregate gate for the point when every
declared consumer is implemented.
