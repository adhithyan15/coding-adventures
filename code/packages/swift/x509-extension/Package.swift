// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "x509-extension",
  products: [.library(name: "X509Extension", targets: ["X509Extension"])],
  dependencies: [
    .package(path: "../der-tlv"),
    .package(path: "../der-asn1"),
  ],
  targets: [
    .target(
      name: "X509Extension",
      dependencies: [.product(name: "DerAsn1", package: "der-asn1")]
    ),
    .testTarget(
      name: "X509ExtensionTests",
      dependencies: [
        "X509Extension",
        .product(name: "DerAsn1", package: "der-asn1"),
        .product(name: "DerTlv", package: "der-tlv"),
      ]
    ),
  ]
)
