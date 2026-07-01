struct ANSIScanner {
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

  mutating func readCSISequence() -> ANSICSISequence? {
    guard peek() == "[" else { return nil }
    advance()

    var parameters = ""
    while let character = nextCharacter() {
      if character.isANSIFinalByte {
        return ANSICSISequence(finalByte: character, parameters: parseANSIParameters(parameters))
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

struct ANSICSISequence {
  let finalByte: Character
  let parameters: [Int]
}

func parseANSIParameters(_ text: String) -> [Int] {
  guard !text.isEmpty else { return [0] }
  return
    text
    .split(separator: ";", omittingEmptySubsequences: false)
    .map { Int($0) ?? 0 }
}

extension Character {
  fileprivate var isANSIFinalByte: Bool { self >= "@" && self <= "~" }
}
