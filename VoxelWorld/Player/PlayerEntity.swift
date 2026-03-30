import Foundation
import Combine

enum Direction: CaseIterable {
    case N, NE, E, SE, S, SW, W, NW

    var dx: Int {
        switch self {
        case .N: return 0
        case .NE: return 1
        case .E: return 1
        case .SE: return 1
        case .S: return 0
        case .SW: return -1
        case .W: return -1
        case .NW: return -1
        }
    }

    var dy: Int {
        switch self {
        case .N: return -1
        case .NE: return -1
        case .E: return 0
        case .SE: return 1
        case .S: return 1
        case .SW: return 1
        case .W: return 0
        case .NW: return -1
        }
    }
}

class PlayerEntity: ObservableObject {
    @Published var gridX: Int = 8
    @Published var gridY: Int = 8
    @Published var facing: Direction = .S
    @Published var inventory: [BlockType] = [.grass, .dirt, .stone, .wood, .leaves]
    @Published var selectedSlot: Int = 0
    var moveCooldown: TimeInterval = 0.15
    var lastMoveTime: Date = .distantPast
    var breakCooldown: TimeInterval = 0.3
    var lastBreakTime: Date = .distantPast

    func canMove() -> Bool {
        return Date().timeIntervalSince(lastMoveTime) >= moveCooldown
    }

    func move(to x: Int, y: Int) {
        gridX = x
        gridY = y
        lastMoveTime = Date()
    }

    func canBreak() -> Bool {
        return Date().timeIntervalSince(lastBreakTime) >= breakCooldown
    }

    func markBroke() {
        lastBreakTime = Date()
    }

    func isAdjacent(to x: Int, y: Int) -> Bool {
        let dx = abs(x - gridX)
        let dy = abs(y - gridY)
        return (dx <= 1 && dy <= 1) && (dx + dy > 0)
    }
}
