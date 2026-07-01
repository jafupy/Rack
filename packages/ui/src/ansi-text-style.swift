import SwiftUI

struct ANSITextStyle {
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
    switch codes[index] {
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
      foreground = ANSITheme.color(codes[index] - 30, bright: false)
    case 38:
      return applyExtendedColor(codes, at: index, to: .foreground)
    case 39:
      foreground = ANSITheme.foreground
    case 40...47:
      background = ANSITheme.color(codes[index] - 40, bright: false)
    case 48:
      return applyExtendedColor(codes, at: index, to: .background)
    case 49:
      background = nil
    case 90...97:
      foreground = ANSITheme.color(codes[index] - 90, bright: true)
    case 100...107:
      background = ANSITheme.color(codes[index] - 100, bright: true)
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

enum ANSIColorTarget {
  case foreground
  case background
}
