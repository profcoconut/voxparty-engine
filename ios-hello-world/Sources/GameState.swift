import Foundation

enum TileType: Int, CaseIterable {
    case void = 0
    case grass = 1
    case dirt = 2
    case stone = 3
    case wood = 4
    case leaves = 5
    case water = 6

    var isWalkable: Bool {
        switch self {
        case .void, .water: return false
        default: return true
        }
    }

    var isPlaceable: Bool {
        switch self {
        case .void: return false
        default: return true
        }
    }
}

struct GameState {
    var world: [[TileType]]
    let gridSize: Int

    init(gridSize: Int = IsometricMath.gridSize) {
        self.gridSize = gridSize
        self.world = Array(repeating: Array(repeating: .void, count: gridSize), count: gridSize)
    }

    static func generate(gridSize: Int) -> GameState {
        var state = GameState(gridSize: gridSize)
        state.world = TerrainGenerator.generate(gridSize: gridSize)
        return state
    }

    mutating func setTile(_ type: TileType, at x: Int, y: Int) {
        guard x >= 0 && x < gridSize && y >= 0 && y < gridSize else { return }
        world[y][x] = type
    }

    func getTile(at x: Int, y: Int) -> TileType {
        guard x >= 0 && x < gridSize && y >= 0 && y < gridSize else { return .void }
        return world[y][x]
    }

    func isWalkable(x: Int, y: Int) -> Bool {
        getTile(at: x, y: y).isWalkable
    }

    var blockCount: Int {
        world.flatMap { $0 }.filter { $0 != .void }.count
    }
}
