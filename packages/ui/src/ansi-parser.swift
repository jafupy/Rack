import SwiftUI

enum ANSIParser {
  static let logBackground = ANSITheme.background

  static func attributedString(_ text: String, fontSize: CGFloat = 10) -> AttributedString {
    var parser = ANSIScanner(text)
    var style = ANSITextStyle(fontSize: fontSize)
    var result = AttributedString()
    var buffer = ""

    func flushBuffer() {
      guard !buffer.isEmpty else { return }
      result += style.attributed(buffer)
      buffer = ""
    }

    while let character = parser.nextCharacter() {
      guard character == ANSIScanner.escape else {
        if character != "\r" { buffer.append(character) }
        continue
      }

      guard let sequence = parser.readCSISequence() else { continue }
      flushBuffer()

      guard sequence.finalByte == "m" else { continue }
      style.applySGRCodes(sequence.parameters)
    }

    flushBuffer()
    return result
  }

  static func strip(_ text: String) -> String {
    var parser = ANSIScanner(text)
    var result = ""

    while let character = parser.nextCharacter() {
      guard character == ANSIScanner.escape else {
        if character != "\r" { result.append(character) }
        continue
      }

      if parser.readCSISequence() == nil {
        result.append(character)
      }
    }

    return result
  }
}
