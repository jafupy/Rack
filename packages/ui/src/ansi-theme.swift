import SwiftUI

// https://github.com/drcmda/poimandres-theme
enum ANSITheme {
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
      return rgb(
        red: colorCubeValue(index / 36),
        green: colorCubeValue((index / 6) % 6),
        blue: colorCubeValue(index % 6)
      )
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

extension Color {
  fileprivate init(hex: UInt32) {
    self.init(
      red: Double((hex >> 16) & 0xff) / 255,
      green: Double((hex >> 8) & 0xff) / 255,
      blue: Double(hex & 0xff) / 255
    )
  }
}
