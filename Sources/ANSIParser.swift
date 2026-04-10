import SwiftUI

/// Parses a string containing ANSI SGR escape codes into an `AttributedString` for display on a dark background.
func ansiAttributedString(_ text: String, fontSize: CGFloat = 10) -> AttributedString {
    var result = AttributedString()

    var fgColor: Color = poimandresFg
    var bgColor: Color? = nil
    var bold = false
    var dim = false

    let baseFont = Font.system(size: fontSize, design: .monospaced)
    let boldFont = Font.system(size: fontSize, weight: .bold, design: .monospaced)

    var buffer = ""
    var i = text.startIndex

    func flush() {
        guard !buffer.isEmpty else { return }
        // Build chunk and set attributes directly on AttributedString —
        // this guarantees SwiftUI-scoped attributes that Text will render.
        var chunk = AttributedString(buffer)
        chunk.font = bold ? boldFont : baseFont
        chunk.foregroundColor = dim ? fgColor.opacity(0.5) : fgColor
        if let bg = bgColor { chunk.backgroundColor = bg }
        result += chunk
        buffer = ""
    }

    while i < text.endIndex {
        let c = text[i]

        // Skip bare carriage returns
        if c == "\r" {
            i = text.index(after: i)
            continue
        }

        // Check for ESC + [  (CSI)
        guard c == "\u{1B}" else {
            buffer.append(c)
            i = text.index(after: i)
            continue
        }

        let afterEsc = text.index(after: i)
        guard afterEsc < text.endIndex, text[afterEsc] == "[" else {
            // Not a CSI sequence — skip the ESC byte
            i = text.index(after: i)
            continue
        }

        flush()
        i = text.index(after: afterEsc) // past ESC[

        // Collect parameter bytes until the final byte (@ through ~)
        var params = ""
        var finalByte: Character = "m"
        while i < text.endIndex {
            let ch = text[i]
            i = text.index(after: i)
            if ch >= "@" && ch <= "~" { finalByte = ch; break }
            params.append(ch)
        }

        guard finalByte == "m" else { continue } // only handle SGR

        let codes: [Int] = params.isEmpty
            ? [0]
            : params.split(separator: ";", omittingEmptySubsequences: false).map { Int($0) ?? 0 }

        var p = 0
        while p < codes.count {
            let code = codes[p]
            switch code {
            case 0:
                fgColor = poimandresFg; bgColor = nil; bold = false; dim = false
            case 1:
                bold = true
            case 2:
                dim = true
            case 22:
                bold = false; dim = false
            case 30...37:
                fgColor = ansiColor(code - 30, bright: false)
            case 38:
                if p + 2 < codes.count, codes[p + 1] == 5 {
                    fgColor = ansi256Color(codes[p + 2]); p += 2
                } else if p + 4 < codes.count, codes[p + 1] == 2 {
                    fgColor = Color(red: Double(codes[p+2])/255, green: Double(codes[p+3])/255, blue: Double(codes[p+4])/255)
                    p += 4
                }
            case 39:
                fgColor = poimandresFg
            case 40...47:
                bgColor = ansiColor(code - 40, bright: false)
            case 48:
                if p + 2 < codes.count, codes[p + 1] == 5 {
                    bgColor = ansi256Color(codes[p + 2]); p += 2
                } else if p + 4 < codes.count, codes[p + 1] == 2 {
                    bgColor = Color(red: Double(codes[p+2])/255, green: Double(codes[p+3])/255, blue: Double(codes[p+4])/255)
                    p += 4
                }
            case 49:
                bgColor = nil
            case 90...97:
                fgColor = ansiColor(code - 90, bright: true)
            case 100...107:
                bgColor = ansiColor(code - 100, bright: true)
            default:
                break
            }
            p += 1
        }
    }

    flush()
    return result
}

/// Strips all ANSI escape sequences, returning plain text.
func stripANSI(_ text: String) -> String {
    var result = ""
    var i = text.startIndex
    while i < text.endIndex {
        let c = text[i]
        if c == "\u{1B}" {
            let next = text.index(after: i)
            if next < text.endIndex, text[next] == "[" {
                i = text.index(after: next)
                while i < text.endIndex {
                    let ch = text[i]; i = text.index(after: i)
                    if ch >= "@" && ch <= "~" { break }
                }
                continue
            }
        }
        result.append(c)
        i = text.index(after: i)
    }
    return result
}

// MARK: - Poimandres theme

// https://github.com/oliveryh/poimandres-theme
private let poimandresFg  = Color(hex: 0xa6accd)
let poimandresBg          = Color(hex: 0x1b1e28)

// MARK: - Color tables

private func ansiColor(_ index: Int, bright: Bool) -> Color {
    switch index {
    case 0: return bright ? Color(hex: 0x767c9d) : Color(hex: 0x1b1e28)
    case 1: return Color(hex: 0xd0679d)
    case 2: return Color(hex: 0x5de4c7)
    case 3: return Color(hex: 0xfffac2)
    case 4: return Color(hex: 0x89ddff)
    case 5: return Color(hex: 0xfae4fc)
    case 6: return Color(hex: 0xadd7ff)
    case 7: return bright ? Color(hex: 0xffffff) : Color(hex: 0xa6accd)
    default: return poimandresFg
    }
}

private extension Color {
    init(hex: UInt32) {
        self.init(
            red:   Double((hex >> 16) & 0xff) / 255,
            green: Double((hex >> 8)  & 0xff) / 255,
            blue:  Double( hex        & 0xff) / 255
        )
    }
}

private func ansi256Color(_ n: Int) -> Color {
    if n < 8  { return ansiColor(n, bright: false) }
    if n < 16 { return ansiColor(n - 8, bright: true) }
    if n < 232 {
        let idx = n - 16
        let b = idx % 6, g = (idx / 6) % 6, r = idx / 36
        func v(_ x: Int) -> Double { x == 0 ? 0 : Double(x * 40 + 55) / 255 }
        return Color(red: v(r), green: v(g), blue: v(b))
    }
    return Color(white: Double((n - 232) * 10 + 8) / 255)
}
