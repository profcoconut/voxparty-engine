import SwiftUI

struct ViewHierarchyView: View {
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HierarchyNode(name: "App", type: "SwiftUI.App", children: [
                    HierarchyNode(name: "ContentView", type: "SwiftUI.View", children: [
                        HierarchyNode(name: "ZStack", type: "SwiftUI.ZStack", children: [
                            HierarchyNode(name: "VStack", type: "SwiftUI.VStack", children: [
                                HierarchyNode(name: "Spacer", type: "SwiftUI.Spacer"),
                                HierarchyNode(name: "Text", type: "SwiftUI.Text", children: [
                                    HierarchyNode(name: "\"Hello World\"", type: "SwiftUI.Text")
                                ])
                            ])
                        ])
                    ])
                ])
            }
            .padding()
        }
    }
}

struct HierarchyNode: View {
    let name: String
    let type: String
    var children: [HierarchyNode] = []
    @State private var isExpanded = true

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 4) {
                if !children.isEmpty {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .frame(width: 12)
                        .onTapGesture {
                            withAnimation {
                                isExpanded.toggle()
                            }
                        }
                } else {
                    Spacer()
                        .frame(width: 12)
                }

                Image(systemName: "cube.fill")
                    .font(.caption)
                    .foregroundColor(.blue)

                Text(name)
                    .font(.system(.caption, design: .monospaced))

                Text(type)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 2)
            .contentShape(Rectangle())

            if isExpanded {
                ForEach(children, id: \.name) { child in
                    HStack(spacing: 0) {
                        Rectangle()
                            .fill(Color.gray.opacity(0.3))
                            .frame(width: 1)
                            .padding(.leading, 6)

                        VStack(alignment: .leading, spacing: 2) {
                            child
                        }
                    }
                }
            }
        }
    }
}

#Preview {
    ViewHierarchyView()
}
