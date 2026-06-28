import SwiftUI

struct SettingsPageLayout<Content: View>: View {
  @ViewBuilder let content: Content

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        content
      }
      .padding(28)
      .frame(maxWidth: 680, alignment: .leading)
    }
  }
}

struct SettingsSectionHeader: View {
  let title: String
  let subtitle: String

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(title)
        .font(.largeTitle.weight(.semibold))
      Text(subtitle)
        .font(.body)
        .foregroundStyle(.secondary)
    }
  }
}

struct SettingsCard<Content: View>: View {
  @ViewBuilder let content: Content

  var body: some View {
    content
      .padding(18)
      .frame(maxWidth: .infinity, alignment: .leading)
      .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
  }
}

struct PlaceholderCard: View {
  let systemImage: String
  let title: String
  let message: String

  var body: some View {
    SettingsCard {
      HStack(alignment: .top, spacing: 14) {
        Image(systemName: systemImage)
          .font(.title2)
          .foregroundStyle(.secondary)
          .frame(width: 32)
        VStack(alignment: .leading, spacing: 6) {
          Text(title)
            .font(.headline)
          Text(message)
            .foregroundStyle(.secondary)
        }
      }
    }
  }
}
