import SwiftUI

// MARK: - Game State Debug View
struct GameStateDebugView: View {
    @ObservedObject var player: PlayerEntity
    @ObservedObject var worldGrid: WorldGrid

    var body: some View {
        List {
            Section("Player") {
                LabeledContent("Grid Position", value: "(\(player.gridX), \(player.gridY))")
                LabeledContent("Facing", value: "\(String(describing: player.facing))")
                LabeledContent("Move Cooldown", value: player.canMove() ? "Ready" : "Cooling")
                LabeledContent("Break Cooldown", value: player.canBreak() ? "Ready" : "Cooling")
            }

            Section("Inventory") {
                ForEach(0..<5) { i in
                    HStack {
                        Text("Slot \(i + 1)")
                        Spacer()
                        RoundedRectangle(cornerRadius: 2)
                            .fill(player.inventory[i].color)
                            .frame(width: 20, height: 20)
                        Text(player.inventory[i].name)
                            .foregroundColor(.secondary)
                    }
                }
                LabeledContent("Selected", value: "Slot \(player.selectedSlot + 1) (\(player.inventory[player.selectedSlot].name))")
            }

            Section("World") {
                LabeledContent("Size", value: "\(worldGrid.width) x \(worldGrid.height)")
                LabeledContent("Total Blocks") {
                    Text("\(countNonAir())")
                }
            }

            Section("Frame Info") {
                LabeledContent("Tile Size", value: "64 x 32 px")
                LabeledContent("Projection", value: "2:1 Isometric")
                LabeledContent("Depth Sort", value: "gridX + gridY ascending")
            }
        }
    }

    func countNonAir() -> Int {
        worldGrid.tiles.flatMap { $0 }.filter { $0 != .air }.count
    }
}

// MARK: - World Grid Debug View
struct WorldGridDebugView: View {
    @ObservedObject var worldGrid: WorldGrid

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 4) {
                Text("World Grid (\(worldGrid.width)x\(worldGrid.height))")
                    .font(.caption.bold())
                    .padding(.bottom, 4)

                ForEach(0..<worldGrid.height, id: \.self) { y in
                    HStack(spacing: 2) {
                        Text(String(format: "%2d", y))
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundColor(.secondary)
                            .frame(width: 20)

                        ForEach(0..<worldGrid.width, id: \.self) { x in
                            let block = worldGrid.getBlock(at: x, y: y)
                            RoundedRectangle(cornerRadius: 1)
                                .fill(block.color)
                                .frame(width: 12, height: 12)
                        }
                    }
                }

                HStack(spacing: 4) {
                    Text("X →")
                        .font(.system(size: 8))
                        .foregroundColor(.secondary)
                }
                .padding(.top, 2)
            }
            .padding()
        }
    }
}

// MARK: - Block Types Debug View
struct BlockTypesDebugView: View {
    var body: some View {
        List {
            ForEach(BlockType.allCases, id: \.self) { block in
                HStack {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(block.color)
                        .frame(width: 30, height: 30)
                        .overlay(
                            RoundedRectangle(cornerRadius: 4)
                                .strokeBorder(Color.primary.opacity(0.3), lineWidth: 1)
                        )

                    VStack(alignment: .leading) {
                        Text(block.name)
                            .font(.headline)
                        Text("Solid: \(block.isSolid ? "Yes" : "No")")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }
}

// MARK: - Input Log Debug View
struct InputLogDebugView: View {
    @State private var logs: [String] = []
    let maxLogs = 20

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Input Log")
                    .font(.caption.bold())
                Spacer()
                Button("Clear") {
                    logs.removeAll()
                }
                .font(.caption)
            }
            .padding(8)
            .background(Color(.systemGray5))

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 2) {
                    ForEach(logs.reversed(), id: \.self) { log in
                        Text(log)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(log.contains("MOVE") ? .blue : (log.contains("BREAK") ? .red : .green))
                    }
                }
                .padding(8)
            }
        }
        .background(Color(.systemBackground))
    }

    mutating func addLog(_ message: String) {
        let timestamp = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
        logs.append("[\(timestamp)] \(message)")
        if logs.count > maxLogs {
            logs.removeFirst()
        }
    }
}

// MARK: - Debug Overlay Main View
struct VoxelDebugOverlay: View {
    @Binding var isPresented: Bool
    @ObservedObject var player: PlayerEntity
    @ObservedObject var worldGrid: WorldGrid

    var body: some View {
        VStack(spacing: 0) {
            // Tab Bar
            HStack(spacing: 0) {
                DebugTabButton(title: "State", isSelected: selectedTab == 0) {
                    selectedTab = 0
                }
                DebugTabButton(title: "World", isSelected: selectedTab == 1) {
                    selectedTab = 1
                }
                DebugTabButton(title: "Blocks", isSelected: selectedTab == 2) {
                    selectedTab = 2
                }
            }
            .padding(.horizontal)
            .padding(.top, 8)

            Divider()
                .padding(.top, 8)

            // Content
            TabView(selection: $selectedTab) {
                GameStateDebugView(player: player, worldGrid: worldGrid)
                    .tag(0)
                WorldGridDebugView(worldGrid: worldGrid)
                    .tag(1)
                BlockTypesDebugView()
                    .tag(2)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
        }
        .frame(maxWidth: .infinity, maxHeight: 400)
        .background(Color(UIColor.systemBackground))
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .shadow(radius: 10)
    }

    @State private var selectedTab = 0
}

struct DebugTabButton: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Text(title)
                    .font(.caption)
                    .fontWeight(isSelected ? .semibold : .regular)
                    .foregroundColor(isSelected ? .blue : .secondary)
                Rectangle()
                    .fill(isSelected ? Color.blue : Color.clear)
                    .frame(height: 2)
            }
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
    }
}

// MARK: - Floating Debug Button
struct VoxelDebugButton: View {
    @Binding var isDebugPresented: Bool

    var body: some View {
        Button(action: {
            isDebugPresented.toggle()
        }) {
            Image(systemName: "ladybug.fill")
                .font(.title3)
                .foregroundColor(.white)
                .frame(width: 44, height: 44)
                .background(Color.red.opacity(0.85))
                .clipShape(Circle())
                .shadow(radius: 4)
        }
    }
}
