import SwiftUI

enum ANSIParser {
  static let logBackground = ANSITheme.background

  /// Parses a string containing ANSI SGR escape codes into an `AttributedString`.
  static func attributedString(_ text: String, fontSize: CGFloat = 10) -> AttributedString {
    var parser = Scanner(text)
    var style = ANSITextStyle(fontSize: fontSize)
    var result = AttributedString()
    var buffer = ""

    func flushBuffer() {
      guard !buffer.isEmpty else { return }
      result += style.attributed(buffer)
      buffer = ""
    }

    while let character = parser.nextCharacter() {
      guard character == Scanner.escape else {
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

  /// Strips ANSI escape sequences, returning plain text.
  static func strip(_ text: String) -> String {
    var parser = Scanner(text)
    var result = ""

    while let character = parser.nextCharacter() {
      guard character == Scanner.escape else {
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

// MARK: - ANSI parsing

private extension ANSIParser {
  struct Scanner {
    static let escape: Character = "\u{1B}"

    private let text: String
    private var index: String.Index

    init(_ text: String) {
      self.text = text
      self.index = text.startIndex
    }

    mutating func nextCharacter() -> Character? {
      guard index < text.endIndex else { return nil }
      let character = text[index]
      index = text.index(after: index)
      return character
    }

    mutating func readCSISequence() -> CSISequence? {
      guard peek() == "[" else { return nil }
      advance()

      var parameters = ""
      while let character = nextCharacter() {
        if character.isANSIFinalByte {
          return CSISequence(finalByte: character, parameters: parseParameters(parameters))
        }
        parameters.append(character)
      }

      return nil
    }

    private func peek() -> Character? {
      index < text.endIndex ? text[index] : nil
    }

    private mutating func advance() {
      index = text.index(after: index)
    }
  }

  struct CSISequence {
    let finalByte: Character
    let parameters: [Int]
  }

  static func parseParameters(_ text: String) -> [Int] {
    guard !text.isEmpty else { return [0] }
    return
      text
      .split(separator: ";", omittingEmptySubsequences: false)
      .map { Int($0) ?? 0 }
  }
}

private extension Character {
  var isANSIFinalByte: Bool { self >= "@" && self <= "~" }
}

// MARK: - Text style

private struct ANSITextStyle {
  var foreground = ANSITheme.foreground
  var background: Color?
  var isBold = false
  var isDim = false

  private let baseFont: Font
  private let boldFont: Font

  init(fontSize: CGFloat) {
    self.baseFont = Font.system(size: fontSize, design: .monospaced)
    self.boldFont = Font.system(size: fontSize, weight: .bold, design: .monospaced)
  }

  func attributed(_ text: String) -> AttributedString {
    var string = AttributedString(text)
    string.font = isBold ? boldFont : baseFont
    string.foregroundColor = isDim ? foreground.opacity(0.5) : foreground
    if let background { string.backgroundColor = background }
    return string
  }

  mutating func applySGRCodes(_ codes: [Int]) {
    var index = 0
    while index < codes.count {
      index += applySGRCode(codes, at: index)
    }
  }

  private mutating func applySGRCode(_ codes: [Int], at index: Int) -> Int {
    let code = codes[index]

    switch code {
    case 0:
      reset()
    case 1:
      isBold = true
    case 2:
      isDim = true
    case 22:
      isBold = false
      isDim = false
    case 30...37:
      foreground = ANSITheme.color(code - 30, bright: false)
    case 38:
      return applyExtendedColor(codes, at: index, to: .foreground)
    case 39:
      foreground = ANSITheme.foreground
    case 40...47:
      background = ANSITheme.color(code - 40, bright: false)
    case 48:
      return applyExtendedColor(codes, at: index, to: .background)
    case 49:
      background = nil
    case 90...97:
      foreground = ANSITheme.color(code - 90, bright: true)
    case 100...107:
      background = ANSITheme.color(code - 100, bright: true)
    default:
      break
    }

    return 1
  }

  private mutating func reset() {
    foreground = ANSITheme.foreground
    background = nil
    isBold = false
    isDim = false
  }

  private mutating func applyExtendedColor(
    _ codes: [Int], at index: Int, to target: ANSIColorTarget
  ) -> Int {
    if index + 2 < codes.count, codes[index + 1] == 5 {
      setColor(ANSITheme.color256(codes[index + 2]), for: target)
      return 3
    }

    if index + 4 < codes.count, codes[index + 1] == 2 {
      setColor(
        ANSITheme.rgb(red: codes[index + 2], green: codes[index + 3], blue: codes[index + 4]),
        for: target)
      return 5
    }

    return 1
  }

  private mutating func setColor(_ color: Color, for target: ANSIColorTarget) {
    switch target {
    case .foreground: foreground = color
    case .background: background = color
    }
  }
}

private enum ANSIColorTarget {
  case foreground
  case background
}

// MARK: - Poimandres theme

// https://github.com/drcmda/poimandres-theme
private enum ANSITheme {
  static let foreground = Color(hex: 0xa6accd)
  static let background = Color(hex: 0x1b1e28)

  static func color(_ index: Int, bright: Bool) -> Color {
    switch index {
    case 0: return bright ? Color(hex: 0x767c9d) : background
    case 1: return Color(hex: 0xd0679d)
    case 2: return Color(hex: 0x5de4c7)
    case 3: return Color(hex: 0xfffac2)
    case 4: return Color(hex: 0x89ddff)
    case 5: return Color(hex: 0xfae4fc)
    case 6: return Color(hex: 0xadd7ff)
    case 7: return bright ? Color(hex: 0xffffff) : foreground
    default: return foreground
    }
  }

  static func color256(_ n: Int) -> Color {
    if n < 8 { return color(n, bright: false) }
    if n < 16 { return color(n - 8, bright: true) }
    if n < 232 {
      let index = n - 16
      let blue = index % 6
      let green = (index / 6) % 6
      let red = index / 36
      return rgb(red: colorCubeValue(red), green: colorCubeValue(green), blue: colorCubeValue(blue))
    }
    return Color(white: Double((n - 232) * 10 + 8) / 255)
  }

  static func rgb(red: Int, green: Int, blue: Int) -> Color {
    Color(red: Double(red) / 255, green: Double(green) / 255, blue: Double(blue) / 255)
  }

  static func rgb(red: Double, green: Double, blue: Double) -> Color {
    Color(red: red, green: green, blue: blue)
  }

  private static func colorCubeValue(_ value: Int) -> Double {
    value == 0 ? 0 : Double(value * 40 + 55) / 255
  }
}

private extension Color {
  init(hex: UInt32) {
    self.init(
      red: Double((hex >> 16) & 0xff) / 255,
      green: Double((hex >> 8) & 0xff) / 255,
      blue: Double(hex & 0xff) / 255
    )
  }
}
