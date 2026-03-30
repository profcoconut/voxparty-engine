import SwiftUI

struct ParticleEffect {
    var x: Int
    var y: Int
    var opacity: Double = 1.0
    var offsetX: CGFloat = 0
    var offsetY: CGFloat = 0
}

struct WorldCanvasView: View {
    @ObservedObject var worldGrid: WorldGrid
    @ObservedObject var player: PlayerEntity
    @State private var particles: [ParticleEffect] = []

    var body: some View {
        GeometryReader { geometry in
            Canvas { context, size in
                let playerScreen = IsometricMath.gridToScreen(x: player.gridX, y: player.gridY)
                let offset = CGSize(
                    width: size.width / 2 - playerScreen.x,
                    height: size.height / 2 - playerScreen.y
                )

                // Collect all tiles with depth keys
                var drawables: [(x: Int, y: Int, depth: Int)] = []
                for x in 0..<worldGrid.width {
                    for y in 0..<worldGrid.height {
                        drawables.append((x: x, y: y, depth: IsometricMath.depthKey(x: x, y: y)))
                    }
                }

                // Sort by depth (ascending)
                drawables.sort { $0.depth < $1.depth }

                // Draw tiles
                for drawable in drawables {
                    let blockType = worldGrid.getBlock(at: drawable.x, y: drawable.y)
                    if blockType != .air {
                        let screenPos = IsometricMath.gridToScreen(x: drawable.x, y: drawable.y)
                        let adjustedPos = CGPoint(
                            x: screenPos.x + offset.width,
                            y: screenPos.y + offset.height
                        )
                        let path = IsometricMath.createDiamondPath(at: adjustedPos)
                        context.fill(path, with: .color(blockType.color))
                    }
                }

                // Draw particles
                for particle in particles {
                    let screenPos = IsometricMath.gridToScreen(x: particle.x, y: particle.y)
                    let adjustedPos = CGPoint(
                        x: screenPos.x + offset.width + particle.offsetX,
                        y: screenPos.y + offset.height + particle.offsetY
                    )
                    let particlePath = Path(ellipseIn: CGRect(x: adjustedPos.x - 3, y: adjustedPos.y - 3, width: 6, height: 6))
                    context.fill(particlePath, with: .color(.white.opacity(particle.opacity)))
                }

                // Draw player
                let playerScreenPos = IsometricMath.gridToScreen(x: player.gridX, y: player.gridY)
                let playerAdjustedPos = CGPoint(
                    x: playerScreenPos.x + offset.width,
                    y: playerScreenPos.y + offset.height
                )
                let playerPath = IsometricMath.createDiamondPath(at: playerAdjustedPos)
                context.fill(playerPath, with: .color(.orange))
                context.stroke(playerPath, with: .color(Color(red: 0.8, green: 0.4, blue: 0)), lineWidth: 2)

            }
            .background(Color(red: 0.1, green: 0.1, blue: 0.2))
            .overlay(alignment: .topLeading) {
                // Coordinate display
                Text("Player: (\(player.gridX), \(player.gridY))")
                    .font(.system(size: 12, weight: .bold, design: .monospaced))
                    .foregroundColor(.white)
                    .padding(8)
                    .background(Color.black.opacity(0.6))
                    .clipShape(RoundedRectangle(cornerRadius: 4))
                    .padding(8)
            }
        }
    }

    func spawnBreakParticles(at x: Int, y: Int) {
        for i in 0..<4 {
            var particle = ParticleEffect(x: x, y: y)
            particle.offsetX = CGFloat.random(in: -10...10)
            particle.offsetY = CGFloat.random(in: -10...10)
            particles.append(particle)
        }

        // Animate particles fading out
        withAnimation(.easeOut(duration: 0.3)) {
            for idx in particles.indices {
                particles[idx].opacity = 0
            }
        }

        // Remove particles after animation
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            particles.removeAll { $0.opacity == 0 }
        }
    }
}
