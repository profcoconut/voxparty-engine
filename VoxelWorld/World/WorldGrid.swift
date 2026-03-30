import Foundation
import SwiftUI
import Combine

class WorldGrid: ObservableObject {
    let width: Int = 16
    let height: Int = 16
    @Published var tiles: [[BlockType]]

    init() {
        tiles = Array(repeating: Array(repeating: .air, count: height), count: width)
        generateTestMap()
    }

    private func generateTestMap() {
        // Checkerboard grass/stone pattern
        for x in 0..<width {
            for y in 0..<height {
                if (x + y) % 2 == 0 {
                    tiles[x][y] = .grass
                } else {
                    tiles[x][y] = .stone
                }
            }
        }

        // Add some dirt patches
        for x in 3..<7 {
            for y in 3..<7 {
                tiles[x][y] = .dirt
            }
        }

        // Add a tree at (12, 4)
        let treeX = 12
        let treeY = 4
        // Tree trunk
        tiles[treeX][treeY] = .wood
        tiles[treeX][treeY - 1] = .wood
        // Leaves
        tiles[treeX - 1][treeY - 2] = .leaves
        tiles[treeX][treeY - 2] = .leaves
        tiles[treeX + 1][treeY - 2] = .leaves
        tiles[treeX - 1][treeY - 3] = .leaves
        tiles[treeX][treeY - 3] = .leaves
        tiles[treeX + 1][treeY - 3] = .leaves

        // Add another tree at (3, 10)
        let tree2X = 3
        let tree2Y = 10
        tiles[tree2X][tree2Y] = .wood
        tiles[tree2X][tree2Y - 1] = .wood
        tiles[tree2X - 1][tree2Y - 2] = .leaves
        tiles[tree2X][tree2Y - 2] = .leaves
        tiles[tree2X + 1][tree2Y - 2] = .leaves
        tiles[tree2X][tree2Y - 3] = .leaves

        // Add water pool
        tiles[10][10] = .water
        tiles[11][10] = .water
        tiles[10][11] = .water
        tiles[11][11] = .water
    }

    func breakBlock(at x: Int, y: Int) {
        guard x >= 0 && x < width && y >= 0 && y < height else { return }
        guard tiles[x][y].isSolid else { return }
        tiles[x][y] = .air
    }

    func placeBlock(_ blockType: BlockType, at x: Int, y: Int) {
        guard x >= 0 && x < width && y >= 0 && y < height else { return }
        guard blockType != .air && !tiles[x][y].isSolid else { return }
        tiles[x][y] = blockType
    }

    func getBlock(at x: Int, y: Int) -> BlockType {
        guard x >= 0 && x < width && y >= 0 && y < height else { return .air }
        return tiles[x][y]
    }
}
