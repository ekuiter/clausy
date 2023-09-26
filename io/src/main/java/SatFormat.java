import de.ovgu.featureide.fm.core.base.FeatureUtils;
import de.ovgu.featureide.fm.core.base.IConstraint;
import de.ovgu.featureide.fm.core.base.IFeature;
import de.ovgu.featureide.fm.core.base.IFeatureModel;
import de.ovgu.featureide.fm.core.editing.NodeCreator;
import de.ovgu.featureide.fm.core.io.AFeatureModelFormat;
import org.prop4j.*;

import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.*;

/**
 * Format for writing DIMACS .sat files.
 */
public class SatFormat extends AFeatureModelFormat {
	private HashMap<String, Integer> variableMap = new HashMap<>();
	private List<String> variables = new ArrayList<>();

	private class SatNodeWriter extends NodeWriter {
		SatNodeWriter(Node root) {
			super(root);
			setNotation(Notation.PREFIX);
			try {
				Field field = NodeWriter.class.getDeclaredField("symbols");
				field.setAccessible(true);
				// nonstandard operators are not supported
				field.set(this, new String[]{"-", "*", "+", "<ERR>", "<ERR>", "<ERR>", "<ERR>", "<ERR>", "<ERR>"});
			} catch (NoSuchFieldException | IllegalAccessException e) {
				e.printStackTrace();
			}
		}

		@Override
		protected String variableToString(Object variable) {
			String name = super.variableToString(variable);
			int index;
			if (variableMap.containsKey(name)) {
				index = variableMap.get(name);
			} else {
				index = variableMap.size() + 1;
				variableMap.put(name, index);
				variables.add(name);
			}
			return String.valueOf(index);
		}

		@Override
		protected void literalToString(Literal l, Class<? extends Node> parent, StringBuilder sb, int depth) {
			if (!l.positive) {
				sb.append(this.getSymbols()[0]);
			}
			sb.append(variableToString(l.var));
		}
	}

	private String getVariableDirectory() {
		StringBuilder sb = new StringBuilder();
        for (int i = 0; i < variables.size(); i++) {
            sb.append("c ");
            sb.append(i + 1);
            sb.append(' ');
            sb.append(variables.get(i));
            sb.append('\n');
        }
		return sb.toString();
	}

	@Override
	public String write(IFeatureModel featureModel) {
		try {
			final IFeature root = FeatureUtils.getRoot(featureModel);
			final List<Node> nodes = new LinkedList<>();
			if (root != null) {
				nodes.add(new Literal(NodeCreator.getVariable(root.getName(), featureModel)));
				Method method = NodeCreator.class.getDeclaredMethod("createNodes", Collection.class, IFeature.class, IFeatureModel.class, boolean.class, Map.class);
				method.setAccessible(true);
				method.invoke(NodeCreator.class, nodes, root, featureModel, true, Collections.emptyMap());
			}
			for (final IConstraint constraint : new ArrayList<>(featureModel.getConstraints())) {
				nodes.add(constraint.getNode().clone());
			}

			StringBuilder sb = new StringBuilder();
			Method method = Node.class.getDeclaredMethod("eliminateNonCNFOperators");
			method.setAccessible(true);
            for (int i = 0; i < nodes.size(); i++) {
                Node node = nodes.get(i);
                // replace nonstandard operators (usually, only AtMost for alternatives) with hardcoded CNF patterns
                node = (Node) method.invoke(node);
                // append constraint to the built .sat file
                sb.append(new SatNodeWriter(node)
						.nodeToString()
						.replace("(* ", "*(")
						.replace("(+ ", "+(")
						.replace("(- ", "-("));
				if (i != nodes.size() - 1)
					sb.append("\n  ");
            }
			return String.format("%sp sat %d\n*(%s)", getVariableDirectory(), variableMap.size(), sb);
		} catch (NoSuchMethodException | InvocationTargetException | IllegalAccessException e) {
			e.printStackTrace();
		}
		return null;
	}

	@Override
	public String getSuffix() {
		return "sat";
	}

	@Override
	public SatFormat getInstance() {
		return this;
	}

	@Override
	public String getId() {
		return SatFormat.class.getCanonicalName();
	}

	@Override
	public boolean supportsWrite() {
		return true;
	}

	@Override
	public String getName() {
		return ".sat";
	}

}
