import SwiftUI

@main
struct VoxelWorldApp: App {
    var body: some Scene {
        WindowGroup {
            GameContentView()
        }
    }
}

struct GameContentView: View {
    @StateObject private var worldGrid = WorldGrid()
    @StateObject private var player = PlayerEntity()
    @StateObject private var coordinator: TouchCoordinator
    @State private var isDebugPresented = false

    init() {
        let grid = WorldGrid()
        let playerr = PlayerEntity()
        _worldGrid = StateObject(wrappedValue: grid)
        _player = StateObject(wrappedValue: playerr)
        _coordinator = StateObject(wrappedValue: TouchCoordinator(worldGrid: grid, player: playerr))
    }

    var body: some View {
        ZStack {
            WorldCanvasView(worldGrid: worldGrid, player: player)
                .ignoresSafeArea()

            VStack {
                Spacer()
                InventoryBarView(player: player)
                    .padding(.bottom, 30)
            }

            // Invisible touch handler overlay
            TouchHandler(worldGrid: worldGrid, player: player) { gridX, gridY in
                coordinator.handleTap(gridX: gridX, gridY: gridY)
            }
            .ignoresSafeArea()

            // Floating debug button
            VStack {
                HStack {
                    Spacer()
                    VoxelDebugButton(isDebugPresented: $isDebugPresented)
                        .padding()
                }
                Spacer()
            }

            // Debug sheet
            if isDebugPresented {
                Color.black.opacity(0.4)
                    .ignoresSafeArea()
                    .onTapGesture {
                        isDebugPresented = false
                    }

                VStack {
                    Spacer()
                    VoxelDebugOverlay(isPresented: $isDebugPresented, player: player, worldGrid: worldGrid)
                        .frame(maxWidth: 400)
                        .padding(.horizontal)
                        .padding(.bottom, 120)
                }
                .transition(.move(edge: .bottom))
            }
        }
        .animation(.spring(response: 0.3), value: isDebugPresented)
    }
}
