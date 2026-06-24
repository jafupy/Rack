import Foundation
import RackUI

struct RackHookSummaryPayload: Decodable {
  let name: String
  let routes: [RackHookRoutePayload]
  let crons: [RackHookCronPayload]
  let errors: [String]
}

struct RackHookRoutePayload: Decodable {
  let method: String
  let path: String
}

struct RackHookCronPayload: Decodable {
  let schedule: String
  let hook: String
}

extension HookSummary {
  init(_ hook: RackHookSummaryPayload) {
    self.init(
      name: hook.name,
      routes: hook.routes.map(HookRoute.init),
      crons: hook.crons.map(HookCron.init),
      errors: hook.errors
    )
  }
}

extension HookRoute {
  init(_ route: RackHookRoutePayload) {
    self.init(method: route.method, path: route.path)
  }
}

extension HookCron {
  init(_ cron: RackHookCronPayload) {
    self.init(schedule: cron.schedule, hook: cron.hook)
  }
}
