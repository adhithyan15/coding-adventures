// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "der-asn1",
  products: [.library(name: "DerAsn1", targets: ["DerAsn1"])],
  dependencies: [.package(path: "../der-tlv")],
  targets: [
    .target(name: "DerAsn1", dependencies: [.product(name: "DerTlv", package: "der-tlv")]),
    .testTarget(
      name: "DerAsn1Tests",
      dependencies: ["DerAsn1", .product(name: "DerTlv", package: "der-tlv")]
    ),
  ]
)
