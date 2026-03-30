import SwiftUI

struct TouchHandler: View {
    @ObservedObject var worldGrid: WorldGrid
    @ObservedObject var player: PlayerEntity
    let onTap: (Int, Int) -> Void

    var body: some View {
        GeometryReader { geometry in
            Color.clear
                .contentShape(Rectangle())
                .onTapGesture { location in
                    handleTap(at: location, in: geometry.size)
                }
        }
    }

    private func handleTap(at location: CGPoint, in size: CGSize) {
        let playerScreen = IsometricMath.gridToScreen(x: player.gridX, y: player.gridY)
        let offset = CGSize(
            width: size.width / 2 - playerScreen.x,
            height: size.height / 2 - playerScreen.y
        )

        let (gridX, gridY) = IsometricMath.screenToGrid(point: location, offset: offset)

        onTap(gridX, gridY)
    }
}

class TouchCoordinator: ObservableObject {
    @Published var worldGrid: WorldGrid
    @Published var player: PlayerEntity

    init(worldGrid: WorldGrid, player: PlayerEntity) {
        self.worldGrid = worldGrid
        self.player = player
    }

    func handleTap(gridX: Int, gridY: Int) {
        // If tapping player's own position, ignore
        if gridX == player.gridX && gridY == player.gridY {
            return
        }

        // If player can break and block is adjacent
        if player.isAdjacent(to: gridX, y: gridY) {
            let block = worldGrid.getBlock(at: gridX, y: gridY)
            if block.isSolid && player.canBreak() {
                worldGrid.breakBlock(at: gridX, y: gridY)
                player.markBroke()
                return
            }
            if !block.isSolid && player.inventory[player.selectedSlot] != .air {
                worldGrid.placeBlock(player.inventory[player.selectedSlot], at: gridX, y: gridY)
                player.inventory[player.selectedSlot] = .air
                return
            }
        }

        // If not adjacent, move player toward tapped location
        if player.canMove() && worldGrid.getBlock(at: gridX, y: gridY).isSolid == false {
            let dx = gridX - player.gridX
            let dy = gridY - player.gridY

            // Move one step toward the target
            var newX = player.gridX
            var newY = player.gridY

            if dx != 0 {
                newX += dx > 0 ? 1 : -1
            }
            if dy != 0 {
                newY += dy > 0 ? 1 : -1
            }

            if !worldGrid.getBlock(at: newX, y: newY).isSolid {
                player.move(to: newX, y: newY)
            }
        }
    }
}
