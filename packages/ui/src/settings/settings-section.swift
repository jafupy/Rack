import Foundation

public enum SettingsSection: String, CaseIterable, Identifiable, Hashable {
  case general
  case services
  case hooks

  public var id: Self { self }

  var title: String {
    switch self {
    case .general: "General"
    case .services: "Services"
    case .hooks: "Hooks"
    }
  }

  var systemImage: String {
    switch self {
    case .general: "gearshape"
    case .services: "server.rack"
    case .hooks: "point.3.connected.trianglepath.dotted"
    }
  }
}
