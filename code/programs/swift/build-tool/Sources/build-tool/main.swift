import BuildToolCore
import Foundation

exit(BuildTool.run(arguments: Array(CommandLine.arguments.dropFirst())))
